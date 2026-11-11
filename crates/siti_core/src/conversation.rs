//! Channel-agnostic conversation/session helpers built on top of generic DB tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

use crate::error::SitiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub channel: String,
    pub external_id: String,
    pub title: Option<String>,
    pub kind: String,
    pub persona: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: i64,
    pub conversation_id: i64,
    pub external_message_id: Option<String>,
    pub user_external_id: Option<String>,
    pub role: String,
    pub text: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

pub async fn ensure_conversation(
    pool: &PgPool,
    channel: &str,
    external_id: &str,
    kind: &str,
    title: Option<&str>,
) -> Result<Conversation, SitiError> {
    let row = sqlx::query_as::<_, (i64, String, String, Option<String>, String, Option<String>, Value, DateTime<Utc>, DateTime<Utc>)>(
        "INSERT INTO conversations (channel, external_id, kind, title)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (channel, external_id) DO UPDATE SET
           kind = EXCLUDED.kind,
           title = COALESCE(EXCLUDED.title, conversations.title),
           updated_at = now()
         RETURNING id, channel, external_id, title, kind, persona, metadata, created_at, updated_at",
    )
    .bind(channel)
    .bind(external_id)
    .bind(kind)
    .bind(title)
    .fetch_one(pool)
    .await?;
    Ok(Conversation {
        id: row.0,
        channel: row.1,
        external_id: row.2,
        title: row.3,
        kind: row.4,
        persona: row.5,
        metadata: row.6,
        created_at: row.7,
        updated_at: row.8,
    })
}

pub async fn get_conversation(pool: &PgPool, id: i64) -> Result<Option<Conversation>, SitiError> {
    let row = sqlx::query_as::<_, (i64, String, String, Option<String>, String, Option<String>, Value, DateTime<Utc>, DateTime<Utc>)>(
        "SELECT id, channel, external_id, title, kind, persona, metadata, created_at, updated_at FROM conversations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| Conversation {
        id: row.0,
        channel: row.1,
        external_id: row.2,
        title: row.3,
        kind: row.4,
        persona: row.5,
        metadata: row.6,
        created_at: row.7,
        updated_at: row.8,
    }))
}

pub async fn list_conversations(pool: &PgPool, channel: Option<&str>, limit: i64) -> Result<Vec<Conversation>, SitiError> {
    let rows = if let Some(channel) = channel {
        sqlx::query_as::<_, (i64, String, String, Option<String>, String, Option<String>, Value, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT id, channel, external_id, title, kind, persona, metadata, created_at, updated_at
             FROM conversations WHERE channel = $1 ORDER BY updated_at DESC LIMIT $2",
        )
        .bind(channel)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, (i64, String, String, Option<String>, String, Option<String>, Value, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT id, channel, external_id, title, kind, persona, metadata, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows
        .into_iter()
        .map(|row| Conversation {
            id: row.0,
            channel: row.1,
            external_id: row.2,
            title: row.3,
            kind: row.4,
            persona: row.5,
            metadata: row.6,
            created_at: row.7,
            updated_at: row.8,
        })
        .collect())
}

pub async fn upsert_participant(
    pool: &PgPool,
    conversation_id: i64,
    user_external_id: &str,
    display_name: &str,
    username: Option<&str>,
    language_code: Option<&str>,
) -> Result<(), SitiError> {
    sqlx::query(
        "INSERT INTO conversation_participants (conversation_id, user_external_id, display_name, username, language_code)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (conversation_id, user_external_id) DO UPDATE SET
           display_name = EXCLUDED.display_name,
           username = EXCLUDED.username,
           language_code = EXCLUDED.language_code,
           updated_at = now()",
    )
    .bind(conversation_id)
    .bind(user_external_id)
    .bind(display_name)
    .bind(username)
    .bind(language_code)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_message(
    pool: &PgPool,
    conversation_id: i64,
    external_message_id: Option<&str>,
    user_external_id: Option<&str>,
    role: &str,
    text: &str,
    metadata: Value,
) -> Result<i64, SitiError> {
    // Note: unique (conversation_id, external_message_id) allows multiple NULLs in Postgres,
    // so inserts without external_message_id always create a new row (no conflict path).
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO conversation_messages (conversation_id, external_message_id, user_external_id, role, text, metadata)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (conversation_id, external_message_id) DO UPDATE SET text = EXCLUDED.text
         RETURNING id",
    )
    .bind(conversation_id)
    .bind(external_message_id)
    .bind(user_external_id)
    .bind(role)
    .bind(text)
    .bind(metadata)
    .fetch_one(pool)
    .await?;
    sqlx::query("UPDATE conversations SET updated_at = now() WHERE id = $1")
        .bind(conversation_id)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn recent_messages(pool: &PgPool, conversation_id: i64, limit: i64) -> Result<Vec<ConversationMessage>, SitiError> {
    let mut rows = sqlx::query_as::<_, (i64, i64, Option<String>, Option<String>, String, String, Value, DateTime<Utc>)>(
        "SELECT id, conversation_id, external_message_id, user_external_id, role, text, metadata, created_at
         FROM conversation_messages WHERE conversation_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.reverse();
    Ok(rows
        .into_iter()
        .map(|r| ConversationMessage {
            id: r.0,
            conversation_id: r.1,
            external_message_id: r.2,
            user_external_id: r.3,
            role: r.4,
            text: r.5,
            metadata: r.6,
            created_at: r.7,
        })
        .collect())
}

pub async fn log_agent_run(
    pool: &PgPool,
    channel: &str,
    conversation_ref: &str,
    user_ref: Option<&str>,
    input_text: &str,
    output_text: Option<&str>,
    tools_used: &[String],
    steps: i32,
    success: bool,
    error: Option<&str>,
    duration_ms: Option<i64>,
) -> Result<(), SitiError> {
    sqlx::query(
        "INSERT INTO agent_runs (channel, conversation_ref, user_ref, input_text, output_text, tools_used, steps, success, error, duration_ms)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(channel)
    .bind(conversation_ref)
    .bind(user_ref)
    .bind(input_text)
    .bind(output_text)
    .bind(tools_used)
    .bind(steps)
    .bind(success)
    .bind(error)
    .bind(duration_ms)
    .execute(pool)
    .await?;
    Ok(())
}