use command::Command;
use pembantu_core::provider::{ImageProvider, TextProvider};
use secrecy::ExposeSecret;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;

use crate::types::Config;

pub mod media_kind;
pub mod command;
pub mod conversation;
pub mod updates;
pub mod types;

struct Handler {
    conversation: conversation::Conversation,
    bot_username: String,   
}

impl Handler {
    async fn answer_command(&self, msg: Message, cmd: Command) -> ResponseResult<()> {
        let from = msg.from.as_ref().map(|v| v.full_name()).unwrap_or("".into());
        log::info!("Replying to command /{} from {}", cmd, from);
        
    
        self.conversation.reply_command(msg, cmd).await
    }
    async fn answer_replied_message(&self, msg: Message) -> ResponseResult<()> {
        let from = msg.from.as_ref().map(|v| v.full_name()).unwrap_or("".into());
        log::info!("[answer_replied_message] Replying to message from {}", from);
        
        if let Some(reply_to_msg) =  msg.reply_to_message() {
            if let Some(user) = &reply_to_msg.from {
                if user.username.as_ref().unwrap() == &self.bot_username {
                    let result = self.conversation.reply_message(msg).await;
                    return result
                }
            } 
        }
        else if msg.text().is_some() || msg.caption().is_some() {
            let text = msg.text().or(msg.caption()).unwrap();
            
            if text.starts_with("AI,") || msg.chat.is_private() {
                // replying to private chat
                let result = self.conversation.reply_message(msg).await;
                return result
            }
        }
    
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    let env = envy::from_env::<Config>().expect("Failed to parse env");
    // init providers
    let text_provider = match env.provider_text_generation {
        None => None,
        Some(s) => Some(match s.as_str() {
            "openrouter" => TextProvider::OpenRouter(env.openrouter_api_key.expose_secret().to_string(), env.model_text_generation.expect("MODEL_TEXT_GENERATION is empty")),
            "gemini" => TextProvider::Gemini(env.gemini_api_key.expose_secret().to_string(), env.model_text_generation.expect("MODEL_TEXT_GENERATION is empty")),
            _ => panic!("Text generation model not supported. Available options: openrouter, gemini")
        })
    };
    let image_provider = match env.provider_image_generation {
        None => None,
        Some(s) => Some(match s.as_str() {
            "gemini" => ImageProvider::Gemini(env.gemini_api_key.expose_secret().to_string(), env.model_image_generation.expect("MODEL_IMAGE_GENERATION is empty")),
            _ => panic!("Image generation model not supported. Available options: gemini")
        })
    };


    let teloxide_bot = Arc::new(teloxide::Bot::from_env());
    let ai = pembantu_core::bot::Bot::new(text_provider, image_provider).unwrap();
    let bot_username = env::var("BOT_USERNAME").expect("BOT_USERNAME should be set");
    let conversation = conversation::Conversation::new(ai, teloxide_bot.clone());

    let handler = Arc::new(Box::new(Handler {
        conversation,
        bot_username
    }));
    let handler_arc2: Arc<Box<Handler>> = handler.clone();
    log::info!("Starting bot..");

    
    let handler = dptree::entry()
        // this branch is to reply to command
        .branch(Update::filter_message().filter_command::<command::Command>().endpoint(move |msg: Message, cmd: Command| {
            let handler = handler.clone();
            async move {
                let chatid = msg.chat.id;
                let from = msg.from.as_ref().map(|v| v.full_name()).unwrap_or("".into()).clone();
                match handler.answer_command(msg.clone(), cmd.clone()).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!(
                            "Failed to process message from user: {:?} in chat: {}. Error: {:?}",
                            from,
                            chatid,
                            e
                        );
                        Err(e)
                    }
                }
            }
        }))
        // this branch is to reply to direct message
        .branch(Update::filter_message().endpoint(move |msg: Message| {
            let handler = handler_arc2.clone();
            async move { 
                let chatid = msg.chat.id;
                let from = msg.from.as_ref().map(|v| v.full_name()).unwrap_or("".into()).clone();
                handler.answer_replied_message(msg).await.map_err(|e| {
                    log::error!(
                        "Failed to process message from user: {:?} in chat: {}. Error: {:?}",
                        from,
                        chatid,
                        e
                    );
                    e
                })
            }
        }));

    Dispatcher::builder(teloxide_bot, handler).enable_ctrlc_handler().build().dispatch().await;
}
