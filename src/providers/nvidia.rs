use super::{Message, Provider};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub struct NvidiaProvider {
    pub api_key: String,
    pub model: String,
    client: Client,
}

impl NvidiaProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            client: Client::new(),
            model: model.unwrap_or_else(|| "z-ai/glm-4.7".to_string()),
        }
    }
}

impl Provider for NvidiaProvider {
    fn chat(
        &self,
        messages: Vec<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        Box::pin(async move {
            let mut body = json!({
                "model": model,
                "messages": messages,
                "max_tokens": 16384,
                "temperature": 1.0,
                "top_p": 0.95,
            });
            // GLM-4.7 supports interleaved thinking — enable it but clear
            // so only the final answer is returned (no raw <think> tokens).
            if model.contains("glm") {
                body["chat_template_kwargs"] = json!({
                    "enable_thinking": true,
                    "clear_thinking": true
                });
            }

            let response = client
                .post("https://integrate.api.nvidia.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                return Err(anyhow::anyhow!("NVIDIA API error: {}", err_text));
            }

            let json_val: serde_json::Value = response.json().await?;
            let content = json_val["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("No response")
                .to_string();

            Ok(content)
        })
    }
}
