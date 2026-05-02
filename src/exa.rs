use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct ExaClient {
    pub api_key: String,
    client: Client,
    last_call: Mutex<Instant>,
}

impl ExaClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            last_call: Mutex::new(Instant::now() - Duration::from_millis(200)),
        }
    }

    async fn rate_limit(&self) {
        let mut last = self.last_call.lock().await;
        let elapsed = last.elapsed();
        // 10 QPS = 100ms minimum between calls, add 20ms buffer
        if elapsed < Duration::from_millis(120) {
            tokio::time::sleep(Duration::from_millis(120) - elapsed).await;
        }
        *last = Instant::now();
    }

    pub async fn search(&self, query: &str) -> Result<String> {
        self.rate_limit().await;

        let body = json!({
            "query": query,
            "type": "fast",
            "numResults": 5,
            "contents": {
                "highlights": true
            }
        });

        let resp = self
            .client
            .post("https://api.exa.ai/search")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Exa {} — {}", status, err));
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(format_results(&data, query))
    }
}

fn format_results(data: &serde_json::Value, query: &str) -> String {
    let mut out = format!("[web search: {}]\n\n", query);
    let Some(results) = data["results"].as_array() else {
        return out + "No results found.\n";
    };

    for (i, r) in results.iter().enumerate() {
        let title = r["title"].as_str().unwrap_or("Untitled");
        let url = r["url"].as_str().unwrap_or("");
        let date = r["publishedDate"].as_str().unwrap_or("");
        let date_part = if date.is_empty() { String::new() } else { format!(" · {}", &date[..10.min(date.len())]) };

        out.push_str(&format!("{}. **{}**{}\n   {}\n", i + 1, title, date_part, url));

        if let Some(highlights) = r["highlights"].as_array() {
            for h in highlights.iter().take(3) {
                if let Some(text) = h.as_str() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        out.push_str(&format!("   > {}\n", trimmed));
                    }
                }
            }
        }
        out.push('\n');
    }
    out
}

/// Full plan-mode chat loop with Exa tool calling.
/// Runs up to 3 search rounds then returns the final answer.
pub async fn plan_chat(
    cerebras_key: String,
    model: String,
    mut messages: Vec<serde_json::Value>,
    exa_key: String,
) -> Result<String> {
    let http = Client::new();
    let exa = ExaClient::new(exa_key);

    let tools = json!([{
        "type": "function",
        "function": {
            "name": "web_search",
            "strict": true,
            "description": "Search the web for current, accurate information. Use for: latest library versions, documentation, best practices, architecture patterns, news, benchmarks, or anything that might have changed since your training cutoff.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "A focused, specific search query."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }
        }
    }]);

    // Tool calling loop — max 3 searches to stay within QPS budget
    for round in 0..3usize {
        let body = if round == 0 {
            json!({
                "model": model,
                "messages": messages,
                "tools": tools,
                "parallel_tool_calls": false,
                "max_tokens": 4096,
                "temperature": 0.7
            })
        } else {
            json!({
                "model": model,
                "messages": messages,
                "tools": tools,
                "parallel_tool_calls": false,
                "max_tokens": 4096,
                "temperature": 0.7
            })
        };

        let resp = http
            .post("https://api.cerebras.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", cerebras_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Cerebras error: {}", err));
        }

        let data: serde_json::Value = resp.json().await?;
        let choice = &data["choices"][0]["message"];

        if let Some(tool_calls) = choice["tool_calls"].as_array() {
            if tool_calls.is_empty() {
                let content = choice["content"].as_str().unwrap_or("").to_string();
                return Ok(content);
            }

            // Append the assistant message (with tool_calls) to history
            messages.push(choice.clone());

            // Execute each tool call
            for tc in tool_calls {
                let call_id = tc["id"].as_str().unwrap_or("").to_string();
                let fn_name = tc["function"]["name"].as_str().unwrap_or("");
                let args_raw = tc["function"]["arguments"].as_str().unwrap_or("{}");

                if fn_name == "web_search" {
                    let args: serde_json::Value =
                        serde_json::from_str(args_raw).unwrap_or_default();
                    let query = args["query"].as_str().unwrap_or("").to_string();
                    let result = exa.search(&query).await.unwrap_or_else(|e| {
                        format!("Search failed: {}", e)
                    });

                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": result
                    }));
                }
            }
            // loop continues → send results back to model
        } else {
            // No tool calls — model gave a final answer
            let content = choice["content"].as_str().unwrap_or("").to_string();
            return Ok(content);
        }
    }

    // Max rounds reached — final call without tools
    let body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": 4096,
        "temperature": 0.7
    });
    let resp = http
        .post("https://api.cerebras.ai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", cerebras_key))
        .json(&body)
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;
    Ok(data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
