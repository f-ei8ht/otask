pub mod cerebras;
pub mod nvidia;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub trait Provider: Send + Sync {
    #[allow(dead_code)]
    fn chat(
        &self,
        messages: Vec<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>;
}
