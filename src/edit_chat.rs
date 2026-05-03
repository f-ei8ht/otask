use crate::tools::{create_file, edit_file, read_file};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub async fn edit_chat(
    cerebras_key: String,
    model: String,
    mut messages: Vec<serde_json::Value>,
    cwd: String,
) -> Result<String> {
    let http = Client::new();

    let tools = json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the full contents of a file before editing or referencing it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative file path from the working directory, e.g. src/main.rs"
                        }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_file",
                "description": "Create a new file (or overwrite an existing one) with the given content. Parent directories are created automatically.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative file path, e.g. src/utils.rs"
                        },
                        "content": {
                            "type": "string",
                            "description": "Full content to write to the file"
                        }
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Edit a file by replacing a unique string with new content. Always read_file first to confirm exact content. Fails if old_str is not found or appears more than once.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative file path"
                        },
                        "old_str": {
                            "type": "string",
                            "description": "Exact string to find (must be unique in the file)"
                        },
                        "new_str": {
                            "type": "string",
                            "description": "Replacement string"
                        }
                    },
                    "required": ["path", "old_str", "new_str"],
                    "additionalProperties": false
                }
            }
        }
    ]);

    // Tool-calling loop — up to 10 rounds (read → edit → verify etc.)
    for _round in 0..10usize {
        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "parallel_tool_calls": false,
            "max_tokens": 8192,
            "temperature": 0.2
        });

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

        // ── Path A: proper OpenAI-format tool_calls ──────────────────────────
        if let Some(tool_calls) = choice["tool_calls"].as_array() {
            if tool_calls.is_empty() {
                return Ok(choice["content"].as_str().unwrap_or("").to_string());
            }

            messages.push(choice.clone());

            for tc in tool_calls {
                let call_id = tc["id"].as_str().unwrap_or("call_0").to_string();
                let fn_name = tc["function"]["name"].as_str().unwrap_or("");
                let args_raw = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let args: serde_json::Value =
                    serde_json::from_str(args_raw).unwrap_or_default();

                let result = execute_tool(fn_name, &args, &cwd);

                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": result
                }));
            }
            continue;
        }

        // ── Path B: text-format fallback (model outputs JSON in content) ─────
        let content = choice["content"].as_str().unwrap_or("").trim().to_string();

        if let Ok(maybe_call) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(name) = maybe_call["name"].as_str() {
                let args = &maybe_call["arguments"];
                let known = matches!(name, "read_file" | "create_file" | "edit_file");
                if known {
                    let result = execute_tool(name, args, &cwd);
                    let call_id = "call_fallback_0";
                    let args_json = serde_json::to_string(args).unwrap_or_default();
                    messages.push(json!({
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": call_id,
                            "type": "function",
                            "function": { "name": name, "arguments": args_json }
                        }]
                    }));
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": result
                    }));
                    continue;
                }
            }
        }

        // ── Path C: genuine final answer ─────────────────────────────────────
        return Ok(content);
    }

    // Max rounds reached — one final call without tools
    let body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": 8192,
        "temperature": 0.2
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

fn execute_tool(name: &str, args: &serde_json::Value, cwd: &str) -> String {
    match name {
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            read_file(path, cwd)
        }
        "create_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            create_file(path, content, cwd)
        }
        "edit_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let old_str = args["old_str"].as_str().unwrap_or("");
            let new_str = args["new_str"].as_str().unwrap_or("");
            edit_file(path, old_str, new_str, cwd)
        }
        _ => format!("Unknown tool: {}", name),
    }
}
