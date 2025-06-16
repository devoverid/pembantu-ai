use std::sync::Arc;

use pembantu_core::error::PembantuError;
use teloxide::{requests::{Requester, ResponseResult}, sugar::request::RequestReplyExt, types::{ChatId, InputFile, InputMedia, InputMediaPhoto, MediaKind, MediaText, Message, MessageKind}};
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

    pub async fn generate_image(&self, prompt: String) -> Result<Vec<u8>, PembantuError> {
        self.bot.generate_image(prompt)
            .await
    }

    pub async fn generate_and_send_text(&self, msg: Message, text: String) -> ResponseResult<()> {
        // Send 'loading' message to user
        let sent_msg = self.teloxide_bot
            .send_message(msg.chat.id, "*Sedang berpikir* ⏳")
            .reply_to(msg.id)
            .await?;

        let response = self.generate_message(text).await;
        let response_str = match response {
            Ok(v) => v,
            Err(e) => {
                log::error!("Error generating message. Error: {:?}", e);
                "Sorry, I am currently experiencing an error. Please contact administrator.".into()
            }
        };
        // Send the response from AI to user
        self.teloxide_bot.edit_message_text(msg.chat.id, sent_msg.id, response_str).await?;

        return Ok(())
    }

    pub async fn generate_and_send_image(&self, msg: Message, text: String) -> ResponseResult<()> {
        // Send 'loading' message to user
        let sent_msg = self.teloxide_bot
            .send_message(msg.chat.id, "Dalam proses.. ⏳")
            .reply_to(msg.id)
            .await?;

        let response = self.generate_image(text).await;
        let response_data = match response {
            Ok(v) => v,
            Err(e) => {
                log::error!("Error generating message. Error: {:?}", e);
                "Sorry, I am currently experiencing an error. Please contact administrator.".into()
            }
        };

        // Update the message when the AI has responded
        self.teloxide_bot.edit_message_media(msg.chat.id, sent_msg.id, InputMedia::Photo(InputMediaPhoto::new(InputFile::memory(response_data)))).await?;

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
                    MessageKind::Common(ref common) => {
                        if let MediaKind::Text(ref text) = common.media_kind {
                            let text_content = text.text.clone();
                            self.generate_and_send_text(msg, text_content).await?;
                        }
                    }
                    _ => {
                        log::info!("Unimplemented: {:?}", msg.kind);
                        unimplemented!()
                    }
                }

            }
            Command::Image => {
                // Get the reply from AI
                match msg.kind {
                    MessageKind::Common(ref common) => {
                        if let MediaKind::Text(ref text) = common.media_kind {
                            let text_content = text.text.clone();
                            self.generate_and_send_image(msg, text_content).await?;
                        }
                    },
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
        if let MessageKind::Common(common) = &msg.kind {
            if let MediaKind::Text(text) = &common.media_kind {
                let text_content = text.text.clone();
                self.generate_and_send_text(msg, text_content).await?;
            }
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