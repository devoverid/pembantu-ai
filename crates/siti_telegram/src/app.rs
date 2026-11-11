use siti_core::{agent::Agent, db::PgPool};
use teloxide::{types::Me, Bot};

/// Shared state handed to every handler as `Arc<App>`.
pub struct App {
    pub bot: Bot,
    pub agent: Agent,
    pub db: PgPool,
    pub me: Me,
}
