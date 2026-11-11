use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use futures::stream::Stream;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::json;
use siti_core::{
    agent::{Agent, AgentEvent, AgentRequest, Attachment, ChatInfo, ChatKind, Trigger, UserInfo},
    conversation,
    db::{self, NewMessage, PgPool},
    knowledge,
    llm::{LlmClient, LlmProvider},
    provider::gemini::types::GeminiAPI,
    tools::web::TavilyClient,
};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    agent: Arc<Agent>,
    db: PgPool,
    config: Arc<RuntimeConfig>,
    rate: Arc<Mutex<RateLimiter>>,
}

#[derive(Clone)]
struct RuntimeConfig {
    api_key: Option<String>, // optional global API key (simple auth)
    require_auth: bool,
    rate_limit_per_minute: u32,
}

#[derive(Debug, Deserialize)]
struct Config {
    database_url: SecretString,
    #[serde(default = "default_host")]
    api_host: String,
    #[serde(default = "default_port")]
    api_port: u16,
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
    /// If set, requests must provide matching `X-API-Key` (or `Authorization: Bearer <key>`).
    #[serde(default)]
    api_key: Option<SecretString>,
    #[serde(default)]
    require_auth: bool,
    #[serde(default = "default_rate")]
    rate_limit_per_minute: u32,
}

fn default_host() -> String { "0.0.0.0".into() }
fn default_port() -> u16 { 3000 }
fn default_timezone() -> String { "Asia/Jakarta".into() }
fn default_rate() -> u32 { 60 }

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
    if search.is_none() {
        log::warn!("TAVILY_API_KEY is not set; web_search and read_webpage are disabled");
    }

    Agent::new(db, llm, search, image, timezone)
}

#[derive(Default)]
struct RateLimiter {
    buckets: HashMap<String, (Instant, u32)>,
}

impl RateLimiter {
    fn allow(&mut self, key: &str, limit: u32) -> bool {
        if limit == 0 { return true; }
        let now = Instant::now();
        let entry = self.buckets.entry(key.to_owned()).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(60) {
            *entry = (now, 0);
        }
        if entry.1 >= limit {
            return false;
        }
        entry.1 += 1;
        true
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse { error: String, code: String }

fn err(status: StatusCode, code: &str, msg: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: msg.into(), code: code.into() }))
}

async fn authorize(state: &AppState, headers: &HeaderMap) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .or_else(|| {
            headers.get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|s| s.to_owned())
        });

    // simple/global auth; can be extended to DB-backed api_keys table
    if let Some(expected) = &state.config.api_key {
        match provided.as_deref() {
            Some(p) if p == expected => {}
            Some(_) => return Err(err(StatusCode::UNAUTHORIZED, "unauthorized", "invalid API key")),
            None if state.config.require_auth => return Err(err(StatusCode::UNAUTHORIZED, "unauthorized", "missing API key")),
            None => {}
        }
    } else if state.config.require_auth {
        return Err(err(StatusCode::UNAUTHORIZED, "unauthorized", "API key required but not configured"));
    }

    let bucket_key = provided.unwrap_or_else(|| "anonymous".into());
    let mut rl = state.rate.lock().await;
    if !rl.allow(&bucket_key, state.config.rate_limit_per_minute) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "rate_limited", "rate limit exceeded"));
    }
    Ok(bucket_key)
}

async fn health() -> impl IntoResponse { Json(json!({ "status": "ok", "service": "siti_api" })) }

// ---- Sessions / Conversations ----
#[derive(Debug, Deserialize)]
struct CreateSessionReq {
    #[serde(default = "default_channel")]
    channel: String,
    /// External conversation id (if omitted, a UUID is generated).
    external_id: Option<String>,
    #[serde(default = "default_user_name")]
    user_name: String,
    user_external_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    language_code: Option<String>,
}
fn default_channel() -> String { "api".into() }
fn default_user_name() -> String { "User".into() }

#[derive(Debug, Serialize)]
struct SessionResp {
    conversation_id: i64,
    channel: String,
    external_id: String,
    kind: String,
    title: Option<String>,
}

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSessionReq>,
) -> Result<Json<SessionResp>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    let external_id = req.external_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let kind = req.kind.unwrap_or_else(|| "private".into());
    let conv = conversation::ensure_conversation(&state.db, &req.channel, &external_id, &kind, req.title.as_deref())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    let user_ext = req.user_external_id.unwrap_or_else(|| "user-1".into());
    conversation::upsert_participant(&state.db, conv.id, &user_ext, &req.user_name, None, req.language_code.as_deref())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    Ok(Json(SessionResp { conversation_id: conv.id, channel: conv.channel, external_id: conv.external_id, kind: conv.kind, title: conv.title }))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    let channel = q.get("channel").map(|s| s.as_str());
    let limit = q.get("limit").and_then(|s| s.parse().ok()).unwrap_or(50);
    let items = conversation::list_conversations(&state.db, channel, limit)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    Ok(Json(json!({ "items": items })))
}

async fn get_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    let conv = conversation::get_conversation(&state.db, id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "not_found", "conversation not found"))?;
    let messages = conversation::recent_messages(&state.db, id, 100)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    Ok(Json(json!({ "conversation": conv, "messages": messages })))
}

// ---- Knowledge ----
#[derive(Debug, Deserialize)]
struct AddKnowledgeReq {
    title: String,
    content: String,
    source: Option<String>,
    owner_user_id: Option<i64>,
    conversation_id: Option<i64>,
}

async fn add_knowledge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AddKnowledgeReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    if req.title.trim().is_empty() || req.content.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "invalid_input", "title and content are required"));
    }
    let id = knowledge::add_document(&state.db, req.owner_user_id, req.conversation_id, &req.title, &req.content, req.source.as_deref())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    Ok(Json(json!({ "id": id })))
}

async fn search_knowledge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    let query = q.get("q").cloned().unwrap_or_default();
    let limit = q.get("limit").and_then(|s| s.parse().ok()).unwrap_or(5);
    let owner = q.get("owner_user_id").and_then(|s| s.parse().ok());
    let docs = knowledge::search_documents(&state.db, &query, owner, limit)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    Ok(Json(json!({ "items": docs })))
}

async fn delete_knowledge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    let deleted = knowledge::delete_document(&state.db, id, None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
    Ok(Json(json!({ "deleted": deleted })))
}

// ---- Chat ----
#[derive(Debug, Deserialize)]
struct ChatRequest {
    /// Either provide `conversation_id` (session) or a raw `chat_id`/`user_id` pair.
    conversation_id: Option<i64>,
    #[serde(default)]
    channel: Option<String>,
    user_id: Option<i64>,
    chat_id: Option<i64>,
    #[serde(default)]
    thread_id: Option<i32>,
    #[serde(default = "default_user_name")]
    user_name: String,
    #[serde(default)]
    user_external_id: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    language_code: Option<String>,
    message: String,
    #[serde(default)]
    chat_title: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct ChatResponse {
    conversation_id: Option<i64>,
    text: String,
    attachments: Vec<AttachmentOut>,
    tools_used: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct AttachmentOut { kind: String, bytes_len: usize, caption: String }

fn synthetic_message_id() -> i32 { (Utc::now().timestamp_millis() & 0x7fff_ffff) as i32 }

async fn resolve_chat(req: &ChatRequest, db: &PgPool) -> Result<(i64, i64, String, Option<i64>), (StatusCode, Json<ErrorResponse>)> {
    // returns (core_user_id, core_chat_id, channel, conversation_id)
    if let Some(cid) = req.conversation_id {
        let conv = conversation::get_conversation(db, cid)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "not_found", "conversation not found"))?;
        // map conversation to core numeric ids deterministically for compatibility
        let core_chat_id = 900_000_000_000i64 + conv.id;
        let user_ext = req.user_external_id.clone().unwrap_or_else(|| "user-1".into());
        let core_user_id = req.user_id.unwrap_or_else(|| 800_000_000_000i64 + (conv.id % 1_000_000));
        conversation::upsert_participant(db, conv.id, &user_ext, &req.user_name, req.username.as_deref(), req.language_code.as_deref())
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "db_error", e.to_string()))?;
        // also ensure core user/chat rows exist for agent memory/history compatibility
        let user = UserInfo { id: core_user_id, full_name: req.user_name.clone(), username: req.username.clone(), language_code: req.language_code.clone() };
        let _ = db::upsert_user(db, &user).await;
        let _ = db::upsert_chat(db, core_chat_id, &conv.kind, conv.title.as_deref()).await;
        // store in conversation messages
        let _ = conversation::add_message(db, conv.id, Some(&Uuid::new_v4().to_string()), Some(&user_ext), "user", &req.message, json!({})).await;
        return Ok((core_user_id, core_chat_id, conv.channel, Some(conv.id)));
    }
    let user_id = req.user_id.ok_or_else(|| err(StatusCode::BAD_REQUEST, "invalid_input", "user_id or conversation_id is required"))?;
    let chat_id = req.chat_id.ok_or_else(|| err(StatusCode::BAD_REQUEST, "invalid_input", "chat_id or conversation_id is required"))?;
    Ok((user_id, chat_id, req.channel.clone().unwrap_or_else(|| "api".into()), None))
}

async fn run_agent(
    state: &AppState,
    req: ChatRequest,
) -> Result<ChatResponse, (StatusCode, Json<ErrorResponse>)> {
    if req.message.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "invalid_input", "message must not be empty"));
    }
    let start = Instant::now();
    let (core_user_id, core_chat_id, channel, conversation_id) = resolve_chat(&req, &state.db).await?;

    let user = UserInfo {
        id: core_user_id,
        full_name: req.user_name.clone(),
        username: req.username.clone(),
        language_code: req.language_code.clone(),
    };
    let _ = db::upsert_user(&state.db, &user).await;
    let chat_kind_str = if req.chat_title.is_some() { "group" } else { "private" };
    let _ = db::upsert_chat(&state.db, core_chat_id, chat_kind_str, req.chat_title.as_deref()).await;
    let msg_id = synthetic_message_id();
    let now = Utc::now();
    let new_msg = NewMessage {
        chat_id: core_chat_id,
        telegram_message_id: msg_id,
        thread_id: req.thread_id,
        user_id: Some(core_user_id),
        role: "user",
        text: &req.message,
        reply_to_telegram_message_id: None,
        created_at: now,
    };
    let _ = db::insert_message(&state.db, &new_msg).await;

    let agent_req = AgentRequest {
        chat: ChatInfo {
            id: core_chat_id,
            thread_id: req.thread_id,
            kind: if req.chat_title.is_some() { ChatKind::Group } else { ChatKind::Private },
            title: req.chat_title.clone(),
        },
        user,
        text: req.message.clone(),
        images: vec![],
        reply_context: None,
        trigger: Trigger::UserMessage { telegram_message_id: msg_id },
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
    let mut tools_used = Vec::new();
    while let Ok(ev) = rx.try_recv() { // drain none now
        if let AgentEvent::ToolStarted { name } = ev { tools_used.push(name); }
    }
    let tools_collector = tokio::spawn(async move {
        let mut used = Vec::new();
        while let Some(ev) = rx.recv().await {
            if let AgentEvent::ToolStarted { name } = ev { used.push(name); }
        }
        used
    });

    let result = state.agent.run(agent_req, tx).await;
    let used = tools_collector.await.unwrap_or_default();
    let duration_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(out) => {
            let asst_id = synthetic_message_id().saturating_add(1);
            let asst_msg = NewMessage {
                chat_id: core_chat_id,
                telegram_message_id: asst_id,
                thread_id: req.thread_id,
                user_id: None,
                role: "assistant",
                text: &out.text,
                reply_to_telegram_message_id: Some(msg_id),
                created_at: Utc::now(),
            };
            let _ = db::insert_message(&state.db, &asst_msg).await;
            if let Some(cid) = conversation_id {
                let _ = conversation::add_message(&state.db, cid, Some(&Uuid::new_v4().to_string()), None, "assistant", &out.text, json!({"tools_used": used})).await;
            }
            let _ = conversation::log_agent_run(&state.db, &channel, &core_chat_id.to_string(), Some(&core_user_id.to_string()), &req.message, Some(&out.text), &used, 0, true, None, Some(duration_ms)).await;
            let attachments = out.attachments.into_iter().map(|a| match a {
                Attachment::Image { bytes, caption } => AttachmentOut { kind: "image".into(), bytes_len: bytes.len(), caption },
            }).collect();
            Ok(ChatResponse { conversation_id, text: out.text, attachments, tools_used: used })
        }
        Err(e) => {
            log::error!("agent run failed: {e}");
            let _ = conversation::log_agent_run(&state.db, &channel, &core_chat_id.to_string(), Some(&core_user_id.to_string()), &req.message, None, &used, 0, false, Some(&e.to_string()), Some(duration_ms)).await;
            Err(err(StatusCode::INTERNAL_SERVER_ERROR, "agent_error", "agent failed to produce a response"))
        }
    }
}

async fn chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    Ok(Json(run_agent(&state, req).await?))
}

/// SSE streaming endpoint: emits events (start, tool, final, error, done).
async fn chat_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers).await?;
    if req.message.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "invalid_input", "message must not be empty"));
    }
    let (tx, rx) = mpsc::unbounded_channel::<Result<Event, Infallible>>();
    let state = state.clone();
    tokio::spawn(async move {
        let _ = tx.send(Ok(Event::default().event("start").data(json!({"status":"running"}).to_string())));
        // We run agent and collect tool events by running in parallel: spawn agent with its own channel and forward.
        // Simpler approach: run_agent collects tools internally but we want live events; so reimplement lightweight run here.
        let (tool_tx, mut tool_rx) = mpsc::unbounded_channel::<AgentEvent>();
        let forward = {
            let tx = tx.clone();
            tokio::spawn(async move {
                while let Some(ev) = tool_rx.recv().await {
                    if let AgentEvent::ToolStarted { name } = ev {
                        let _ = tx.send(Ok(Event::default().event("tool_started").data(json!({"name": name}).to_string())));
                    }
                }
            })
        };
        // run core path by duplicating minimal logic via run_agent is not easy since it consumes tx; so call agent directly here.
        let start = Instant::now();
        let resolved = resolve_chat(&req, &state.db).await;
        match resolved {
            Err(e) => {
                let _ = tx.send(Ok(Event::default().event("error").data(json!({"error": e.1.error}).to_string())));
            }
            Ok((core_user_id, core_chat_id, channel, conversation_id)) => {
                let user = UserInfo { id: core_user_id, full_name: req.user_name.clone(), username: req.username.clone(), language_code: req.language_code.clone() };
                let _ = db::upsert_user(&state.db, &user).await;
                let chat_kind_str = if req.chat_title.is_some() { "group" } else { "private" };
                let _ = db::upsert_chat(&state.db, core_chat_id, chat_kind_str, req.chat_title.as_deref()).await;
                let msg_id = synthetic_message_id();
                let _ = db::insert_message(&state.db, &NewMessage { chat_id: core_chat_id, telegram_message_id: msg_id, thread_id: req.thread_id, user_id: Some(core_user_id), role: "user", text: &req.message, reply_to_telegram_message_id: None, created_at: Utc::now() }).await;
                let agent_req = AgentRequest { chat: ChatInfo { id: core_chat_id, thread_id: req.thread_id, kind: if req.chat_title.is_some() { ChatKind::Group } else { ChatKind::Private }, title: req.chat_title.clone() }, user, text: req.message.clone(), images: vec![], reply_context: None, trigger: Trigger::UserMessage { telegram_message_id: msg_id } };
                match state.agent.run(agent_req, tool_tx).await {
                    Ok(out) => {
                        let _ = forward.await;
                        let asst_id = synthetic_message_id().saturating_add(1);
                        let _ = db::insert_message(&state.db, &NewMessage { chat_id: core_chat_id, telegram_message_id: asst_id, thread_id: req.thread_id, user_id: None, role: "assistant", text: &out.text, reply_to_telegram_message_id: Some(msg_id), created_at: Utc::now() }).await;
                        if let Some(cid) = conversation_id { let _ = conversation::add_message(&state.db, cid, Some(&Uuid::new_v4().to_string()), None, "assistant", &out.text, json!({})).await; }
                        let _ = conversation::log_agent_run(&state.db, &channel, &core_chat_id.to_string(), Some(&core_user_id.to_string()), &req.message, Some(&out.text), &[], 0, true, None, Some(start.elapsed().as_millis() as i64)).await;
                        let attachments: Vec<_> = out.attachments.iter().map(|a| match a { Attachment::Image { bytes, caption } => json!({"kind":"image","bytes_len":bytes.len(),"caption":caption}) }).collect();
                        let _ = tx.send(Ok(Event::default().event("final").data(json!({"text": out.text, "attachments": attachments, "conversation_id": conversation_id}).to_string())));
                    }
                    Err(e) => {
                        let _ = forward.await;
                        let _ = conversation::log_agent_run(&state.db, &channel, &core_chat_id.to_string(), Some(&core_user_id.to_string()), &req.message, None, &[], 0, false, Some(&e.to_string()), Some(start.elapsed().as_millis() as i64)).await;
                        let _ = tx.send(Ok(Event::default().event("error").data(json!({"error":"agent failed"}).to_string())));
                    }
                }
            }
        }
        let _ = tx.send(Ok(Event::default().event("done").data("{}")));
    });
    let stream = UnboundedReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();
    let config: Config = envy::from_env().expect("failed to read configuration from the environment");

    let db = db::connect(config.database_url.expose_secret())
        .await
        .unwrap_or_else(|e| panic!("failed to connect to DATABASE_URL: {e}"));
    let agent = Arc::new(build_agent(&config, db.clone()));
    let runtime = Arc::new(RuntimeConfig {
        api_key: non_empty_secret(config.api_key.clone()).map(|s| s.expose_secret().to_owned()),
        require_auth: config.require_auth,
        rate_limit_per_minute: config.rate_limit_per_minute,
    });

    let state = AppState { agent, db, config: runtime, rate: Arc::new(Mutex::new(RateLimiter::default())) };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/sessions", post(create_session).get(list_sessions))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/knowledge", post(add_knowledge).get(search_knowledge))
        .route("/v1/knowledge/{id}", delete(delete_knowledge))
        .route("/v1/chat", post(chat))
        .route("/v1/chat/stream", post(chat_stream))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.api_host, config.api_port).parse().expect("invalid API_HOST/API_PORT");
    log::info!("siti_api listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind failed");
    axum::serve(listener, app).await.expect("server error");
}
