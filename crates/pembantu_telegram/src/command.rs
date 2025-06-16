use std::fmt;

use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "Tanya apapun ke pembantu")]
    Ask,
    #[command(description = "Buat gambar dengan AI")]
    Image,
    #[command(description = "Bantuan menggunakan bot ini")]
    Help,
}


impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Ask => write!(f, "ask"),
            Command::Image => write!(f, "image"),
            Command::Help => write!(f, "help"),
        }
    }
}