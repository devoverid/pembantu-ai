use std::sync::Arc;

use pembantu_core::error::PembantuError;
use teloxide::{requests::{Requester, ResponseResult}, sugar::request::RequestReplyExt, types::{ChatId, MediaKind, MediaText, Message, MessageKind}};
use crate::command::Command;


pub struct Conversation {
    bot: Arc<pembantu_core::bot::Bot>,
    teloxide_bot: Arc<teloxide::Bot>
}
impl Conversation {
    pub fn new(bot: Arc<pembantu_core::bot::Bot>, teloxide_bot: Arc<teloxide::Bot>) -> Self {
        Self {
            bot,
            teloxide_bot,
        }
    }

    pub async fn generate_message(&self, text: String) -> Result<String, PembantuError> {
        self.bot.generate_text(text)
            .await
    }

    pub async fn generate_and_send_message(&self,  chat_id: ChatId, text: MediaText) -> ResponseResult<()> {
        // Send 'loading' message to user
        let sent_msg = self.teloxide_bot.send_message(chat_id, "*Sedang berpikir* ⏳").await?;

        let response = self.generate_message(text.text).await;
        let response_str = match response {
            Ok(v) => v,
            Err(e) => {
                log::error!("Error generating message. Error: {:?}", e);
                "Sorry, I am currently experiencing an error. Please contact administrator.".into()
            }
        };

        // Update the message when the AI has responded
        self.teloxide_bot.edit_message_text(chat_id, sent_msg.id, response_str).await?;

        Ok(())
    }

    pub async fn reply_command(&self, msg: Message, cmd: Command) -> ResponseResult<()> {
        match cmd {
            Command::Help => {
                self.teloxide_bot.send_message(msg.chat.id, "Bantuan: ketik /ask untuk bertanya").await?;
            },
            Command::Ask => {
                // Get the reply from AI
                match msg.kind {
                    MessageKind::Common(common) => {
                        if let MediaKind::Text(text) = common.media_kind {
                            self.generate_and_send_message(msg.chat.id, text).await?;
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

    pub async fn reply_message(&self, msg: Message) -> Result<(), teloxide::RequestError> {

        // Get the reply from AI
        match msg.kind {
            MessageKind::Common(common) => {
                if let MediaKind::Text(text) = common.media_kind {
                    // Send 'loading' message to user
                    let sent_msg = self.teloxide_bot
                        .send_message(msg.chat.id, "*Sedang berpikir* ⏳")
                        .reply_to(msg.id)
                        .await?;
                    let response = self.generate_message(text.text).await;
                    let response_str = match response {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Error generating message. Error: {:?}", e);
                            "Sorry, I am currently experiencing an error. Please contact administrator.".into()
                        }
                    };
                    // Send the response from AI to user
                    self.teloxide_bot.edit_message_text(msg.chat.id, sent_msg.id, response_str).await?;
                }
            },
            _ => unimplemented!()
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use std::env;
    use pembantu_core::provider::{openrouter::types::CompletionsResponse, TextProvider};
    use dotenv::dotenv;

    #[actix_rt::test]
    async fn test_openrouter() {
        dotenv().ok();

        let api_key = env::var("OPENROUTER_API").unwrap();
        let model = "google/gemini-2.0-flash-001".to_string();
        let bot = pembantu_core::bot::Bot::new(Some(TextProvider::OpenRouter(api_key, model)), None).unwrap();
        
        let result = bot.generate_text("Hi, how are you?".into()).await;
        assert!(result.is_ok())
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
        assert!(msg.is_ok())
    }

}