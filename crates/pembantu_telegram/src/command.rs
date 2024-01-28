use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
pub enum Command {
    #[command(description = "Tanya apapun ke pembantu")]
    Ask,
}   
