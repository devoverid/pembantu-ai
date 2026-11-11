use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use siti_core::{
    agent::{Agent, AgentEvent, AgentRequest, ChatInfo, ChatKind, Trigger, UserInfo},
    conversation,
    db::{self, NewMessage, PgPool},
    llm::{LlmClient, LlmProvider},
    provider::gemini::types::GeminiAPI,
    tools::web::TavilyClient,
};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    agent: Arc<Agent>,
    db: PgPool,
    http: reqwest::Client,
    config: Arc<WaConfig>,
}

#[derive(Clone)]
struct WaConfig {
    verify_token: String,
    access_token: Option<String>,
    phone_number_id: Option<String>,
    graph_api_version: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    database_url: SecretString,
    #[serde(default = "default_host")]
    wa_host: String,
    #[serde(default = "default_port")]
    wa_port: u16,
    #[serde(default = "default_timezone")]
    default_timezone: String,
    #[serde(default)]
    provider_text_generation: String,
    #[serde(default)]
    model_text_generation: String,
    #[serde(default)]
    openrouter_api_key: Option<SecretString>,
    #[serde(default)]
    gemini_api_key: Option<SecretString>,
    #[serde(default)]
    tavily_api_key: Option<SecretString>,
    #[serde(default)]
    provider_image_generation: Option<String>,
    #[serde(default)]
    model_image_generation: Option<String>,
    /// Meta webhook verification token (echoed back on challenge).
    #[serde(default)]
    whatsapp_verify_token: String,
    /// Meta Graph API access token (required to send replies).
    #[serde(default)]
    whatsapp_access_token: Option<SecretString>,
    /// WhatsApp Business phone number id (required to send replies).
    #[serde(default)]
    whatsapp_phone_number_id: Option<String>,
    #[serde(default = "default_graph_ver")]
    whatsapp_graph_version: String,
}

fn default_host() -> String { "0.0.0.0".into() }
fn default_port() -> u16 { 8080 }
fn default_timezone() -> String { "Asia/Jakarta".into() }
fn default_graph_ver() -> String { "v20.0".into() }

fn non_empty(value: Option<String>) -> Option<String> { value.filter(|v| !v.trim().is_empty()) }
fn non_empty_secret(value: Option<SecretString>) -> Option<SecretString> {
    value.filter(|v| !v.expose_secret().trim().is_empty())
}

fn build_agent(config: &Config, db: PgPool) -> Agent {
    let openrouter_key = non_empty_secret(config.openrouter_api_key.clone());
    let gemini_key = non_empty_secret(config.gemini_api_key.clone());
    let model = non_empty(Some(config.model_text_generation.clone()))
        .expect("MODEL_TEXT_GENERATION is required");

    let llm = match config.provider_text_generation.as_str() {
        "openrouter" => {
            let key = openrouter_key.as_ref().expect("OPENROUTER_API_KEY is required when PROVIDER_TEXT_GENERATION=openrouter");
            LlmClient::new(LlmProvider::OpenRouter, key.expose_secret().to_owned(), model)
        }
        "gemini" => {
            let key = gemini_key.as_ref().expect("GEMINI_API_KEY is required when PROVIDER_TEXT_GENERATION=gemini");
            LlmClient::new(LlmProvider::Gemini, key.expose_secret().to_owned(), model)
        }
        other => panic!("unknown PROVIDER_TEXT_GENERATION={other:?}; expected openrouter or gemini"),
    };

    let image = match non_empty(config.provider_image_generation.clone()).as_deref() {
        None => None,
        Some("gemini") => {
            let model = non_empty(config.model_image_generation.clone())
                .expect("MODEL_IMAGE_GENERATION is required when PROVIDER_IMAGE_GENERATION is set");
            let key = gemini_key.as_ref().expect("GEMINI_API_KEY is required when PROVIDER_IMAGE_GENERATION=gemini");
            Some(GeminiAPI::new(key.expose_secret().to_owned(), model))
        }
        Some(other) => panic!("unknown PROVIDER_IMAGE_GENERATION={other:?}; expected gemini"),
    };

    let timezone: chrono_tz::Tz = config.default_timezone.parse().unwrap_or_else(|_| {
        panic!("invalid DEFAULT_TIMEZONE={:?}; use an IANA name such as Asia/Jakarta", config.default_timezone)
    });
    let search = non_empty_secret(config.tavily_api_key.clone())
        .map(|key| TavilyClient::new(key.expose_secret().to_owned()));
    Agent::new(db, llm, search, image, timezone)
}

#[derive(Debug, Deserialize)]
struct VerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

async fn verify_webhook(State(state): State<AppState>, Query(q): Query<VerifyQuery>) -> impl IntoResponse {
    if q.mode.as_deref() == Some("subscribe")
        && q.verify_token.as_deref() == Some(state.config.verify_token.as_str())
    {
        if let Some(ch) = q.challenge {
            return (StatusCode::OK, ch);
        }
    }
    (StatusCode::FORBIDDEN, "verification failed".to_string())
}

/// Minimal WhatsApp Cloud API webhook payload structures.
#[derive(Debug, Deserialize)]
struct WebhookPayload {
    #[serde(default)]
    entry: Vec<Entry>,
}
#[derive(Debug, Deserialize)]
struct Entry { #[serde(default)] changes: Vec<Change> }
#[derive(Debug, Deserialize)]
struct Change { value: Option<ChangeValue> }
#[derive(Debug, Deserialize)]
struct ChangeValue {
    #[serde(default)]
    messages: Vec<WaMessage>,
    #[serde(default)]
    contacts: Vec<WaContact>,
}
#[derive(Debug, Deserialize)]
struct WaMessage {
    from: String,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    text: Option<WaText>,
}
#[derive(Debug, Deserialize)]
struct WaText { body: String }
#[derive(Debug, Deserialize)]
struct WaContact { profile: Option<WaProfile>, wa_id: Option<String> }
#[derive(Debug, Deserialize)]
struct WaProfile { name: Option<String> }

async fn receive_webhook(State(state): State<AppState>, Json(payload): Json<WebhookPayload>) -> impl IntoResponse {
    // Ack immediately per WhatsApp requirements; process asynchronously.
    for entry in payload.entry {
        for change in entry.changes {
            if let Some(value) = change.value {
                let contact_name = value
                    .contacts
                    .first()
                    .and_then(|c| c.profile.as_ref())
                    .and_then(|p| p.name.clone())
                    .unwrap_or_else(|| "WhatsApp User".into());
                for msg in value.messages {
                    if msg.kind == "text" {
                        if let Some(text) = msg.text.map(|t| t.body) {
                            let st = state.clone();
                            let from = msg.from.clone();
                            let msg_id = msg.id.clone();
                            let name = contact_name.clone();
                            tokio::spawn(async move {
                                if let Err(e) = process_message(st, from, name, msg_id, text).await {
                                    log::error!("failed to process whatsapp message: {e}");
                                }
                            });
                        }
                    }
                }
            }
        }
    }
    (StatusCode::OK, Json(json!({"status":"received"})))
}

fn synthetic_message_id() -> i32 { (Utc::now().timestamp_millis() & 0x7fff_ffff) as i32 }

async fn process_message(state: AppState, from: String, name: String, wa_msg_id: String, text: String) -> Result<(), String> {
    // Conversation identity: channel=whatsapp, external_id = sender phone (1:1 chat)
    let conv = conversation::ensure_conversation(&state.db, "whatsapp", &from, "private", Some(&name))
        .await
        .map_err(|e| e.to_string())?;
    conversation::upsert_participant(&state.db, conv.id, &from, &name, None, None)
        .await
        .map_err(|e| e.to_string())?;
    let _ = conversation::add_message(&state.db, conv.id, Some(&wa_msg_id), Some(&from), "user", &text, json!({}))
        .await;

    // Map to core ids for agent/memory compatibility
    let core_chat_id = 700_000_000_000i64 + conv.id;
    let core_user_id = 600_000_000_000i64 + conv.id; // 1:1 mapping per conversation for scaffold
    let user = UserInfo { id: core_user_id, full_name: name.clone(), username: None, language_code: None };
    let _ = db::upsert_user(&state.db, &user).await;
    let _ = db::upsert_chat(&state.db, core_chat_id, "private", Some(&name)).await;
    let msg_id = synthetic_message_id();
    let _ = db::insert_message(&state.db, &NewMessage { chat_id: core_chat_id, telegram_message_id: msg_id, thread_id: None, user_id: Some(core_user_id), role: "user", text: &text, reply_to_telegram_message_id: None, created_at: Utc::now() }).await;

    let agent_req = AgentRequest {
        chat: ChatInfo { id: core_chat_id, thread_id: None, kind: ChatKind::Private, title: Some(name) },
        user,
        text,
        images: vec![],
        reply_context: None,
        trigger: Trigger::UserMessage { telegram_message_id: msg_id },
    };
    let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    match state.agent.run(agent_req, tx).await {
        Ok(out) => {
            let reply_text = if out.text.trim().is_empty() { "…".to_string() } else { out.text };
            let asst_id = synthetic_message_id().saturating_add(1);
            let _ = db::insert_message(&state.db, &NewMessage { chat_id: core_chat_id, telegram_message_id: asst_id, thread_id: None, user_id: None, role: "assistant", text: &reply_text, reply_to_telegram_message_id: Some(msg_id), created_at: Utc::now() }).await;
            let _ = conversation::add_message(&state.db, conv.id, Some(&Uuid::new_v4().to_string()), None, "assistant", &reply_text, json!({})).await;
            send_whatsapp_text(&state, &from, &reply_text).await?;
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

async fn send_whatsapp_text(state: &AppState, to: &str, body: &str) -> Result<(), String> {
    let (token, phone_id) = match (&state.config.access_token, &state.config.phone_number_id) {
        (Some(t), Some(p)) => (t, p),
        _ => {
            log::warn!("WHATSAPP_ACCESS_TOKEN/WHATSAPP_PHONE_NUMBER_ID not set; reply not sent (would send to {to}): {body}");
            return Ok(());
        }
    };
    let url = format!("https://graph.facebook.com/{}/{}/messages", state.config.graph_api_version, phone_id);
    let payload = json!({
        "messaging_product": "whatsapp",
        "to": to,
        "type": "text",
        "text": { "body": body }
    });
    let res = state.http.post(url).bearer_auth(token).json(&payload).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("whatsapp send failed {status}: {body}"));
    }
    Ok(())
}

async fn health() -> impl IntoResponse { Json(json!({"status":"ok","service":"siti_whatsapp"})) }

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();
    let config: Config = envy::from_env().expect("failed to read configuration from the environment");

    let db = db::connect(config.database_url.expose_secret())
        .await
        .unwrap_or_else(|e| panic!("failed to connect to DATABASE_URL: {e}"));
    let agent = Arc::new(build_agent(&config, db.clone()));
    let wa = Arc::new(WaConfig {
        verify_token: config.whatsapp_verify_token.clone(),
        access_token: non_empty_secret(config.whatsapp_access_token.clone()).map(|s| s.expose_secret().to_owned()),
        phone_number_id: config.whatsapp_phone_number_id.clone(),
        graph_api_version: config.whatsapp_graph_version.clone(),
    });
    if wa.access_token.is_none() || wa.phone_number_id.is_none() {
        log::warn!("WhatsApp send credentials not fully configured; incoming messages will be processed but replies may not be delivered");
    }

    let state = AppState { agent, db, http: reqwest::Client::new(), config: wa };
    let app = Router::new()
        .route("/health", get(health))
        .route("/webhook", get(verify_webhook).post(receive_webhook))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.wa_host, config.wa_port).parse().expect("invalid WA_HOST/WA_PORT");
    log::info!("siti_whatsapp listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind failed");
    axum::serve(listener, app).await.expect("server error");
}
