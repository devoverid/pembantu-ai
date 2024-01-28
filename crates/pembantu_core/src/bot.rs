use async_trait::async_trait;

pub enum BotKind {
    OpenRouter(String)
}

#[async_trait]
pub trait Bot {
    async fn generate(&self, message: String) -> String;
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