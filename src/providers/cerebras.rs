use super::{Message, Provider};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub struct CerebrasProvider {
    api_key: String,
    client: Client,
    model: String,
}

impl CerebrasProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            model: "llama-3.3-70b".to_string(),
        }
    }
}

impl Provider for CerebrasProvider {
    fn chat(
        &self,
        messages: Vec<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        Box::pin(async move {
            let body = json!({
                "model": model,
                "messages": messages,
                "max_tokens": 4096,
                "temperature": 0.7
            });

            let response = client
                .post("https://api.cerebras.ai/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                return Err(anyhow::anyhow!("Cerebras API error: {}", err_text));
            }

            let json: serde_json::Value = response.json().await?;
            let content = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("No response")
                .to_string();

            Ok(content)
        })
    }
}
