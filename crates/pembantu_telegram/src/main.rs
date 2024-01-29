use command::Command;
use pembantu_core::bot::BotKind;
use teloxide::prelude::*;
use teloxide::types::*;
use dotenv::dotenv;
use std::env;
use std::rc::Rc;
use std::sync::Arc;

pub mod media_kind;
pub mod command;
pub mod conversation;


async fn answer(bot: teloxide::Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    log::info!("Replying to command");
    let bot_kind = BotKind::OpenRouter(env::var("OPENROUTER_API").unwrap());
    let convo = conversation::Conversation::new(bot_kind);
    convo.reply_command(bot, msg, cmd).await?;

    Ok(())
}
#[tokio::main]
async fn main() {
    dotenv().ok();
    
    pretty_env_logger::init();
    log::info!("Starting bot..");
    
    let bot = teloxide::Bot::from_env();
    command::Command::repl(bot,  answer).await;
}
