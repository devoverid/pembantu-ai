use pembantu_core::{bot::{BotKind, Bot}};
use teloxide::{payloads::{SendMessage, SendMessageSetters}, requests::{JsonRequest, Requester, ResponseResult}, types::{ChatId, MediaKind, MediaText, Message, MessageKind}};
use crate::command::Command;


#[derive(Clone)]
pub struct Conversation {
    bot: Box<dyn Bot>
}
impl Conversation {
    pub fn new(bot_kind: BotKind, model_name: String) -> Self {
        Self {
            bot: bot_kind.create_bot_instance(model_name)
        }
    }

    pub async fn generate_message(&self, text: String) -> String {
        self.bot.generate(text)
            .await
            .unwrap_or("Sorry, I am currently experiencing an error. Please contact administrator.".into())
    }

    pub async fn generate_and_send_message(&self, bot: teloxide::Bot, chat_id: ChatId, text: MediaText) -> ResponseResult<()> {
        // Send 'loading' message to user
        let sent_msg = bot.send_message(chat_id, "*Sedang berpikir* ⏳").await?;

        let response = self.generate_message(text.text).await;

        // Update the message when the AI has responded
        bot.edit_message_text(chat_id, sent_msg.id, response).await?;

        Ok(())
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
                            self.generate_and_send_message(bot, msg.chat.id, text).await?;
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
                    // Send 'loading' message to user
                    let new_msg = SendMessage::new(msg.chat.id, "*Sedang berpikir* ⏳")
                        .reply_to_message_id(msg.id);
                    let sent_msg = JsonRequest::new(bot.clone(), new_msg).await?;
                    let response = self.generate_message(text.text).await;

                    // Send the response from AI to user
                    bot.edit_message_text(msg.chat.id, sent_msg.id, response).await?;
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
        let api = OpenRouterAPI::new(api_key, "google/gemini-2.0-flash-001".into());
        
        let result = api.generate("Hi, how are you?".into()).await;
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