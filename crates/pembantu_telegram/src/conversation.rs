use pembantu_core::bot::BotKind;
use teloxide::{requests::ResponseResult, types::Message, Bot};
use crate::command::Command;
use std::env;

pub struct Conversation {
    api_key: String
}

impl Conversation {
    pub fn new(bot_kind: BotKind) -> Self {
        Self {
            api_key: match bot_kind {
                BotKind::OpenRouter(api_key) => api_key
            }
        }
    }

    pub async fn reply_command(&self, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
        match cmd {
            Command::Ask => {
                Ok(())
            }
        }
    }

    pub async fn reply_message(&self) {
        
    }
}


#[cfg(test)]
mod tests {
    use std::env;
    use pembantu_core::bot::Bot;
    use pembantu_core::api::openrouter::{CompletionsResponse, OpenRouterAPI};
    use dotenv::dotenv;

    #[actix_rt::test]
    async fn test_openrouter() {
        dotenv().ok();

        let api_key = env::var("OPENROUTER_API").unwrap();
        let api = OpenRouterAPI::new(api_key);
        
        let result = api.generate("Hi, how are you?".into()).await;
        assert_eq!(result.is_ok(), true)
    }


    #[actix_rt::test]
    async fn test_decode() {
        let msg: &str = r#"{
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "I'm just a computer program, so I don't have feelings. But I'm here and ready to help answer your questions to the best of my ability! How can I assist you today?"
                    },
                    "finish_reason": "stop"
                }
            ],
            "model": "mistralai/mistral-medium",
            "usage": {
                "prompt_tokens": 12,
                "total_tokens": 54,
                "completion_tokens": 42
            },
            "id": "gen-xasrtrast",
            "object": "chat.completion",
            "created": 1706494166
        }"#;

        let msg = serde_json::from_str::<CompletionsResponse>(msg);
        assert_eq!(msg.is_ok(), true)
    }

}