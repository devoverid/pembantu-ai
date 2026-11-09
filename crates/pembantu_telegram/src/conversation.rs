use std::sync::Arc;

use pembantu_core::agent::{AgentRequest, ChatInfo, ChatKind, Trigger, UserInfo};
use teloxide::{requests::{Requester, ResponseResult}, sugar::request::RequestReplyExt, types::{InputFile, InputMedia, InputMediaPhoto, MediaKind, Message, MessageKind}};
use crate::command::Command;


pub struct Conversation {
    bot: pembantu_core::agent::Agent,
    teloxide_bot: Arc<teloxide::Bot>
}
impl Conversation {
    pub fn new(bot: pembantu_core::agent::Agent, teloxide_bot: Arc<teloxide::Bot>) -> Self {
        Self {
            bot,
            teloxide_bot,
        }
    }

    pub async fn generate_and_send_text(&self, msg: Message, text: String) -> ResponseResult<()> {
        // Send 'loading' message to user
        let sent_msg = self.teloxide_bot
            .send_message(msg.chat.id, "*Sedang berpikir* ⏳")
            .reply_to(msg.id)
            .await?;

        let user = msg.from.as_ref();
        let request = AgentRequest {
            chat: ChatInfo {
                id: msg.chat.id.0,
                thread_id: None,
                kind: if msg.chat.is_private() { ChatKind::Private } else { ChatKind::Group },
                title: msg.chat.title().map(str::to_owned),
            },
            user: UserInfo {
                id: user.map(|u| u.id.0 as i64).unwrap_or_default(),
                full_name: user.map(|u| u.full_name()).unwrap_or_default(),
                username: user.and_then(|u| u.username.clone()),
                language_code: user.and_then(|u| u.language_code.clone()),
            },
            text,
            images: Vec::new(),
            reply_context: None,
            trigger: Trigger::UserMessage { telegram_message_id: msg.id.0 },
        };
        let (events, _) = tokio::sync::mpsc::unbounded_channel();
        let response = self.bot.run(request, events).await.map(|output| output.text);
        let response_str = match response {
            Ok(v) => v,
            Err(e) => {
                log::error!("Error generating message. Error: {:?}", e);
                "Sorry, I am currently experiencing an error. Please contact administrator.".into()
            }
        };
        // Send the response from AI to user
        self.teloxide_bot.edit_message_text(msg.chat.id, sent_msg.id, response_str).await?;

        Ok(())
    }

    pub async fn generate_and_send_image(&self, msg: Message, text: String) -> ResponseResult<()> {
        // Send 'loading' message to user
        let sent_msg = self.teloxide_bot
            .send_message(msg.chat.id, "Dalam proses.. ⏳")
            .reply_to(msg.id)
            .await?;

        let response = self.bot.generate_image(&text).await;
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
                self.reply_message(msg).await?;
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
        log::info!("replying message..");
        match msg.kind {
            MessageKind::Common(ref common) => {
                // INSERT_YOUR_CODE
                match common.media_kind {
                    MediaKind::Text(ref text) => {
                        let text_content = text.text.clone();
                        self.generate_and_send_text(msg, text_content).await?;
                    },
                    MediaKind::Photo(ref photo) => {
                        let photo_content = photo.caption.clone();

                        // caption is required
                        if photo_content.is_none() {
                            self.generate_and_send_text(msg, "Please add a caption!".into()).await?;
                            return Ok(());
                        }

                        self.generate_and_send_text(msg, photo_content.clone().unwrap()).await?;
                    },
                    _ => ()
                }
            }
            _ => {
                log::info!("Unimplemented: {:?}", msg.kind);
                unimplemented!()
            }
        }

        Ok(())
    }
}