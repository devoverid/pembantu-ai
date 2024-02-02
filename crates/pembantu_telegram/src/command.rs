use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Start using this BOT")]
    Start,
    #[command(description = "Ask anything to pembantu")]
    Ask,
    #[command(description = "Show help message")]
    Help,
}
