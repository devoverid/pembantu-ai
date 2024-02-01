use pembantu_core::{api::openrouter::OpenRouterAPI, bot::{BotKind, Bot}, error::PembantuError};
use teloxide::{requests::{Request, Requester, ResponseResult}, types::{ChatKind, MediaKind, Message, MessageKind}};
use crate::command::Command;
use std::{borrow::Borrow, env};

#[derive(Clone)]
pub struct Conversation {
    bot: Box<dyn Bot>
}
impl Conversation {
    pub fn new(bot_kind: BotKind) -> Self {
        Self {
            bot: bot_kind.create_bot_instance()
        }
    }

    pub async fn reply_command(&self, bot: teloxide::Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
        log::info!("Replying to command");
        match cmd {
            Command::Help => {
                bot.send_message(msg.chat.id, "Bantuan: ketik /ask untuk bertanya").await?;
            },
            Command::Ask => {

                // Get the reply from AI
                match msg.kind {
                    MessageKind::Common(common) => {
                        if let MediaKind::Text(text) = common.media_kind {

                            // Send 'loading' message to user
                            let sent_msg = bot.send_message(msg.chat.id, "*Sedang berpikir* ⏳").await?;

                            let response = self.bot.generate(text.text)
                                .await
                                .unwrap_or("Sorry, I am currently experiencing an error. Please contact administrator.".into());

                            // Update the message when the AI has responded
                            bot.edit_message_text(msg.chat.id, sent_msg.id, response).await?;
                        }
                    }
                    _ => {
                        log::info!("Unimplemented: {:?}", msg.kind);
                        unimplemented!()
                    }
                }

            }
        }
        Ok(())
    }

    pub async fn reply_message(&self, bot: teloxide::Bot, msg: Message) -> Result<(), teloxide::RequestError> {

        // Get the reply from AI
        match msg.kind {
            MessageKind::Common(common) => {
                if let MediaKind::Text(text) = common.media_kind {
                    let response = self.bot.generate(text.text)
                        .await
                        .unwrap_or("Sorry, I am currently experiencing an error. Please contact administrator.".into());

                    // Send the response from AI to user
                    bot.send_message(msg.chat.id, response).await?;
                }
            }
            _ => unimplemented!()
        }
        Ok(())
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