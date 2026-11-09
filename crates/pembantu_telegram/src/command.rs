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
    #[command(description = "Ingatan saya tentangmu / What I remember about you: /memory [clear | forget <id>]")]
    Memory(String),
    #[command(description = "Ringkas obrolan / Summarize the chat: /summary [jam/hours, default 24]")]
    Summary(String),
}
