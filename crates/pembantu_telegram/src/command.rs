use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Perintah / Commands:")]
pub enum Command {
    #[command(description = "Mulai / Start")]
    Start,
    #[command(description = "Bantuan / Help")]
    Help,
    #[command(description = "Tanya apa saja / Ask anything")]
    Ask(String),
    #[command(description = "Buat gambar / Generate an image")]
    Image(String),
}
