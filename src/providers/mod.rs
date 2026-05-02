pub mod anthropic;
pub mod cerebras;
pub mod codex;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    Cerebras,
    Anthropic,
    Codex,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Cerebras => write!(f, "Cerebras"),
            ProviderType::Anthropic => write!(f, "Anthropic"),
            ProviderType::Codex => write!(f, "Codex"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub trait Provider: Send + Sync {
    fn chat(
        &self,
        messages: Vec<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>;
}
