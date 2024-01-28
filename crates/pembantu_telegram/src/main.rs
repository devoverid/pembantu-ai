use pembantu_core::bot::BotKind;
use teloxide::prelude::*;
use teloxide::types::*;
use dotenv::dotenv;
use std::env;

pub mod media_kind;
pub mod command;
pub mod conversation;



#[tokio::main]
async fn main() {
    dotenv().ok();
    
    pretty_env_logger::init();
    let bot_kind = BotKind::OpenRouter(env::var("OPENROUTER_API").unwrap());
    let mut convo = conversation::Conversation::new(bot_kind);
    println!("token: {}", env::var("OPENROUTER_API").unwrap());
    // let bot = Bot::from_env();
    // command::Command::repl(bot, convo.reply_command());
}
