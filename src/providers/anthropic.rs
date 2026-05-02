use super::{Message, Provider};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub struct AnthropicProvider {
    api_key: String,
    client: Client,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            model: "claude-opus-4-5".to_string(),
        }
    }
}

impl Provider for AnthropicProvider {
    fn chat(
        &self,
        messages: Vec<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        Box::pin(async move {
            let anthropic_messages: Vec<serde_json::Value> = messages
                .iter()
                .filter(|m| m.role != "system")
                .map(|m| {
                    json!({
                        "role": m.role,
                        "content": m.content
                    })
                })
                .collect();

            let system_prompt = messages
                .iter()
                .find(|m| m.role == "system")
                .map(|m| m.content.clone())
                .unwrap_or_else(|| "You are a helpful AI coding assistant.".to_string());

            let body = json!({
                "model": model,
                "max_tokens": 4096,
                "system": system_prompt,
                "messages": anthropic_messages
            });

            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                return Err(anyhow::anyhow!("Anthropic API error: {}", err_text));
            }

            let json: serde_json::Value = response.json().await?;
            let content = json["content"][0]["text"]
                .as_str()
                .unwrap_or("No response")
                .to_string();

            Ok(content)
        })
    }
}
