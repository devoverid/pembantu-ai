use pembantu_core::agent::Agent;
use teloxide::{types::Me, Bot};

/// Shared state handed to every handler as `Arc<App>`.
pub struct App {
    pub bot: Bot,
    pub agent: Agent,
    pub me: Me,
}
