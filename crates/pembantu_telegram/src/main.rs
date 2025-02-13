use command::Command;
use pembantu_core::bot::BotKind;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;

pub mod media_kind;
pub mod command;
pub mod conversation;
pub mod updates;

#[derive(Clone)]
struct Handler {
    conversation: conversation::Conversation,
    bot: teloxide::Bot,
    bot_username: String,   
}

impl Handler {
    async fn answer_command(&self, msg: Message, cmd: Command) -> ResponseResult<()> {
        log::info!("Replying to command");
        let result = self.conversation.reply_command(self.bot.clone(), msg, cmd).await;
    
        result
    }
    async fn answer_replied_message(&self, msg: Message) -> ResponseResult<()> {
        log::info!("Received message");
    
        if let Some(reply_to_msg) =  msg.reply_to_message() {
            if let Some(user) = reply_to_msg.from() {
                if user.username.as_ref().unwrap() == &self.bot_username {
                    let result = self.conversation.reply_message(self.bot.clone(), msg).await;
                    return result
                }
            } 
        }
        else if msg.text().is_some() {
            let text = msg.text().unwrap();
            
            if text.starts_with("AI,") ||
                msg.chat.is_private() {
                // replying to private chat
                let result = self.conversation.reply_message(self.bot.clone(), msg).await;
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
    log::info!("Starting bot..");
    
    let bot = teloxide::Bot::from_env();
    
    let bot_kind = BotKind::OpenRouter(env::var("OPENROUTER_API").expect("OPENROUTER_API should be set"));
    let model = env::var("AI_MODEL").expect("AI_MODEL should be set");
    let bot_username = env::var("BOT_USERNAME").expect("BOT_USERNAME should be set");
    let conversation = conversation::Conversation::new(bot_kind, model);

    let handler = Arc::new(Box::new(Handler {
        conversation,
        bot: bot.clone(),
        bot_username
    }));
    let handler_arc2 = handler.clone();
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

    Dispatcher::builder(bot, handler).enable_ctrlc_handler().build().dispatch().await;
}
