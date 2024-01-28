use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::new("6772037775:AAHycBIvO5I4-0z0YbJPLLLkplvJDc0PWuk");

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_(msg.chat.id).await?;
        Ok(())
    })
    .await;
}