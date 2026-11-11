use std::sync::Arc;

use siti_core::{
    agent::Agent,
    db::{self, PgPool},
    llm::{LlmClient, LlmProvider},
    provider::gemini::types::GeminiAPI,
    tools::web::TavilyClient,
};
use secrecy::{ExposeSecret, SecretString};
use teloxide::{prelude::*, utils::command::BotCommands};

use crate::{app::App, command::Command, types::Config};

mod app;
mod command;
mod handlers;
mod i18n;
mod render;
mod reply;
mod scheduler;
mod types;

/// envy maps `KEY=` to `Some("")`; treat that as unset.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

fn non_empty_secret(value: Option<SecretString>) -> Option<SecretString> {
    value.filter(|v| !v.expose_secret().trim().is_empty())
}

fn build_agent(config: Config, db: PgPool) -> Agent {
    let openrouter_key = non_empty_secret(config.openrouter_api_key);
    let gemini_key = non_empty_secret(config.gemini_api_key);
    let model = non_empty(Some(config.model_text_generation)).expect("MODEL_TEXT_GENERATION is required");

    let llm = match config.provider_text_generation.as_str() {
        "openrouter" => {
            let key = openrouter_key
                .as_ref()
                .expect("OPENROUTER_API_KEY is required when PROVIDER_TEXT_GENERATION=openrouter");
            LlmClient::new(LlmProvider::OpenRouter, key.expose_secret().to_owned(), model)
        }
        "gemini" => {
            let key = gemini_key
                .as_ref()
                .expect("GEMINI_API_KEY is required when PROVIDER_TEXT_GENERATION=gemini");
            LlmClient::new(LlmProvider::Gemini, key.expose_secret().to_owned(), model)
        }
        other => panic!("unknown PROVIDER_TEXT_GENERATION={other:?}; expected openrouter or gemini"),
    };

    let image = match non_empty(config.provider_image_generation).as_deref() {
        None => None,
        Some("gemini") => {
            let model = non_empty(config.model_image_generation)
                .expect("MODEL_IMAGE_GENERATION is required when PROVIDER_IMAGE_GENERATION is set");
            let key = gemini_key
                .as_ref()
                .expect("GEMINI_API_KEY is required when PROVIDER_IMAGE_GENERATION=gemini");
            Some(GeminiAPI::new(key.expose_secret().to_owned(), model))
        }
        Some(other) => panic!("unknown PROVIDER_IMAGE_GENERATION={other:?}; expected gemini"),
    };

    let timezone: chrono_tz::Tz = config.default_timezone.parse().unwrap_or_else(|_| {
        panic!(
            "invalid DEFAULT_TIMEZONE={:?}; use an IANA name such as Asia/Jakarta",
            config.default_timezone
        )
    });

    let search = non_empty_secret(config.tavily_api_key).map(|key| TavilyClient::new(key.expose_secret().to_owned()));
    if search.is_none() {
        log::warn!("TAVILY_API_KEY is not set; web_search and read_webpage are disabled");
    }

    Agent::new(db, llm, search, image, timezone)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();
    let config = envy::from_env::<Config>().expect("failed to read configuration from the environment");

    let bot = Bot::new(config.teloxide_token.expose_secret());
    let db = db::connect(config.database_url.expose_secret())
        .await
        .unwrap_or_else(|e| panic!("failed to connect to DATABASE_URL: {e}"));
    let agent = build_agent(config, db.clone());

    let me = bot.get_me().await.expect("getMe failed; check TELOXIDE_TOKEN");
    if let Err(e) = bot.set_my_commands(Command::bot_commands()).await {
        log::error!("failed to register bot commands: {e}");
    }
    if !me.can_read_all_group_messages {
        log::warn!("Privacy mode is enabled; disable it in @BotFather (/setprivacy) so the bot can read group messages");
    }
    log::info!("Starting @{}", me.username());

    let app = Arc::new(App { bot: bot.clone(), agent, db, me });
    tokio::spawn(scheduler::run(app.clone()));
    let handler = Update::filter_message().endpoint(handlers::handle_message);
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![app])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
