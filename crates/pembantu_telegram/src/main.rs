use command::Command;
use pembantu_core::provider::{ImageProvider, TextProvider};
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
    bot: Arc<teloxide::Bot>,
    bot_username: String,   
}

impl Handler {
    async fn answer_command(&self, msg: Message, cmd: Command) -> ResponseResult<()> {
        log::info!("Replying to command");
        let result = self.conversation.reply_command(msg, cmd).await;
    
        result
    }
    async fn answer_replied_message(&self, msg: Message) -> ResponseResult<()> {
        log::info!("Received message");
    
        if let Some(reply_to_msg) =  msg.reply_to_message() {
            if let Some(user) = &reply_to_msg.from {
                if user.username.as_ref().unwrap() == &self.bot_username {
                    let result = self.conversation.reply_message(msg).await;
                    return result
                }
            } 
        }
        else if msg.text().is_some() {
            let text = msg.text().unwrap();
            
            if text.starts_with("AI,") ||
                msg.chat.is_private() {
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
    let text_provider = match env.model_text_generation {
        None => None,
        Some(s) => Some(match s.as_str() {
            "openrouter" => TextProvider::OpenRouter(env.openrouter_api_key, s),
            _ => panic!("Text generation model not supported. Available options: openrouter, gemini")
        })
    };
    let image_provider = match env.model_image_generation {
        None => None,
        Some(s) => Some(match s.as_str() {
            "gemini" => ImageProvider::Gemini(env.gemini_api_key, s),
            _ => panic!("Image generation model not supported. Available options: gemini")
        })
    };

    
    let teloxide_bot = Arc::new(teloxide::Bot::from_env());
    let ai = pembantu_core::bot::Bot::new(text_provider, image_provider).unwrap();
    let bot_username = env::var("BOT_USERNAME").expect("BOT_USERNAME should be set");
    let conversation = conversation::Conversation::new(Arc::new(ai), teloxide_bot.clone());

    let handler = Arc::new(Box::new(Handler {
        conversation,
        bot: teloxide_bot.clone(),
        bot_username
    }));
    let handler_arc2: Arc<Box<Handler>> = handler.clone();
    log::info!("Starting bot..");

    
    let handler = dptree::entry()
        .branch(Update::filter_message().filter_command::<command::Command>().endpoint(move |msg: Message, cmd: Command| {
            let handler = handler.clone();
            async move {
                handler.answer_command(msg.clone(), cmd.clone()).await
            }
        }))
        .branch(Update::filter_message().endpoint(move |msg| {
            let handler = handler_arc2.clone();
            async move { 
                handler.answer_replied_message(msg).await 
            }
        }));

    Dispatcher::builder(teloxide_bot, handler).enable_ctrlc_handler().build().dispatch().await;
}
