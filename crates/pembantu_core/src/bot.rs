use std::ops::Deref;

use async_trait::async_trait;
use dyn_clone::DynClone;

use crate::{api::openrouter::OpenRouterAPI, error::PembantuError};

pub enum BotKind {
    OpenRouter(String)
}

impl BotKind {
    pub fn create_bot_instance(&self, model_name: String) -> Box<dyn Bot> {
        match self {
            BotKind::OpenRouter(api_key) => Box::new(
                OpenRouterAPI::new(api_key.deref().to_string(), model_name)
            )
        }
    }
}

#[async_trait]
pub trait Bot: Send + Sync + DynClone {
    async fn generate(&self, message: String) -> Result<String, PembantuError>;
}

impl Clone for Box<dyn Bot> {
    fn clone(&self) -> Box<dyn Bot> {
        dyn_clone::clone(self)
    }
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