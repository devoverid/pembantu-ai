use command::Command;
use pembantu_core::bot::BotKind;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use dotenv::dotenv;
use std::env;

pub mod media_kind;
pub mod command;
pub mod conversation;
pub mod updates;


async fn answer_command(bot: teloxide::Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    log::info!("Replying to command");
    let bot_kind = BotKind::OpenRouter(env::var("OPENROUTER_API").unwrap());
    let convo = conversation::Conversation::new(bot_kind);
    convo.reply_command(bot, msg, cmd).await?;

    Ok(())
}
async fn answer_replied_message(bot: teloxide::Bot, msg: Message) -> ResponseResult<()> {
    let bot_kind = BotKind::OpenRouter(env::var("OPENROUTER_API").unwrap());
    let convo = conversation::Conversation::new(bot_kind);
    let bot_username = env::var("BOT_USERNAME").expect("BOT_USERNAME should be set");
    log::info!("Replying to a normal message");

    if let Some(reply_to_msg) =  msg.reply_to_message() {
        if let Some(user) = reply_to_msg.from() {
            if user.username.as_ref().unwrap() == &bot_username {
                convo.reply_message(bot, msg).await?;
            }
        } 
    }
    else if msg.text().is_some() {
        let text = msg.text().unwrap();

        if text.starts_with("AI,") {
            // replying to private chat
            convo.reply_message(bot, msg).await?;
        }
    }

    Ok(())
}
#[tokio::main]
async fn main() {
    dotenv().ok();
    
    pretty_env_logger::init();
    log::info!("Starting bot..");
    
    let bot = teloxide::Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().filter_command::<command::Command>().endpoint(answer_command))
        .branch(Update::filter_message().endpoint(answer_replied_message));
    
    Dispatcher::builder(bot, handler).enable_ctrlc_handler().build().dispatch().await;
}
