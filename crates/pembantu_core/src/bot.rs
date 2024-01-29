use async_trait::async_trait;

use crate::error::PembantuError;

pub enum BotKind {
    OpenRouter(String)
}

#[async_trait]
pub trait Bot {
    async fn generate(&self, message: String) -> Result<String, PembantuError>;
}

pub enum MessageRole {
    Bot,
    User
}

pub struct Message {
    context: String,
    role: MessageRole,
    content: String
}