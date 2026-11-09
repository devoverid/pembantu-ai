//! Postgres persistence. Queries are runtime-checked (no `query!` macros) so builds need no database.

use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
pub use sqlx::PgPool;

use crate::{agent::UserInfo, error::PembantuError};

pub async fn connect(url: &str) -> Result<PgPool, PembantuError> {
    let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|e| PembantuError::Config(e.to_string()))?;
    Ok(pool)
}

pub async fn upsert_user(pool: &PgPool, user: &UserInfo) -> Result<(), PembantuError> {
    sqlx::query(
        "INSERT INTO users (id, username, full_name, language_code) VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username, full_name = EXCLUDED.full_name,
           language_code = EXCLUDED.language_code, updated_at = now()",
    )
    .bind(user.id)
    .bind(&user.username)
    .bind(&user.full_name)
    .bind(&user.language_code)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_chat(pool: &PgPool, id: i64, kind: &str, title: Option<&str>) -> Result<(), PembantuError> {
    sqlx::query(
        "INSERT INTO chats (id, kind, title) VALUES ($1, $2, $3)
         ON CONFLICT (id) DO UPDATE SET kind = EXCLUDED.kind, title = EXCLUDED.title, updated_at = now()",
    )
    .bind(id)
    .bind(kind)
    .bind(title)
    .execute(pool)
    .await?;
    Ok(())
}

pub struct NewMessage<'a> {
    pub chat_id: i64,
    pub telegram_message_id: i32,
    pub thread_id: Option<i32>,
    pub user_id: Option<i64>,
    pub role: &'a str,
    pub text: &'a str,
    pub reply_to_telegram_message_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

pub async fn insert_message(pool: &PgPool, m: &NewMessage<'_>) -> Result<(), PembantuError> {
    sqlx::query(
        "INSERT INTO messages (chat_id, telegram_message_id, thread_id, user_id, role, text,
           reply_to_telegram_message_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (chat_id, telegram_message_id) DO NOTHING",
    )
    .bind(m.chat_id)
    .bind(m.telegram_message_id)
    .bind(m.thread_id)
    .bind(m.user_id)
    .bind(m.role)
    .bind(m.text)
    .bind(m.reply_to_telegram_message_id)
    .bind(m.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoredMessage {
    pub role: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub full_name: Option<String>,
}

/// The newest `limit` messages of a chat/topic, oldest first.
pub async fn recent_messages(
    pool: &PgPool,
    chat_id: i64,
    thread_id: Option<i32>,
    exclude_telegram_message_id: Option<i32>,
    limit: i64,
) -> Result<Vec<StoredMessage>, PembantuError> {
    let mut rows: Vec<StoredMessage> = sqlx::query_as(
        "SELECT m.role, m.text, m.created_at, u.full_name
         FROM messages m LEFT JOIN users u ON u.id = m.user_id
         WHERE m.chat_id = $1 AND m.thread_id IS NOT DISTINCT FROM $2
           AND m.telegram_message_id IS DISTINCT FROM $3
         ORDER BY m.created_at DESC, m.id DESC LIMIT $4",
    )
    .bind(chat_id)
    .bind(thread_id)
    .bind(exclude_telegram_message_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.reverse();
    Ok(rows)
}

/// Messages of a chat/topic since `since`, oldest first; only the newest `limit` are kept.
pub async fn messages_since(
    pool: &PgPool,
    chat_id: i64,
    thread_id: Option<i32>,
    since: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<StoredMessage>, PembantuError> {
    let mut rows: Vec<StoredMessage> = sqlx::query_as(
        "SELECT m.role, m.text, m.created_at, u.full_name
         FROM messages m LEFT JOIN users u ON u.id = m.user_id
         WHERE m.chat_id = $1 AND m.thread_id IS NOT DISTINCT FROM $2 AND m.created_at >= $3
         ORDER BY m.created_at DESC, m.id DESC LIMIT $4",
    )
    .bind(chat_id)
    .bind(thread_id)
    .bind(since)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.reverse();
    Ok(rows)
}

pub async fn user_timezone(pool: &PgPool, user_id: i64) -> Result<Option<String>, PembantuError> {
    let tz: Option<Option<String>> = sqlx::query_scalar("SELECT timezone FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(tz.flatten())
}

pub async fn set_user_timezone(pool: &PgPool, user_id: i64, timezone: &str) -> Result<(), PembantuError> {
    sqlx::query("UPDATE users SET timezone = $2, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .bind(timezone)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_memory(pool: &PgPool, user_id: i64, content: &str) -> Result<i64, PembantuError> {
    Ok(sqlx::query_scalar("INSERT INTO memories (user_id, content) VALUES ($1, $2) RETURNING id")
        .bind(user_id)
        .bind(content)
        .fetch_one(pool)
        .await?)
}

pub async fn count_memories(pool: &PgPool, user_id: i64) -> Result<i64, PembantuError> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM memories WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?)
}

/// The newest `limit` memories of a user, newest first.
pub async fn list_memories(pool: &PgPool, user_id: i64, limit: i64) -> Result<Vec<(i64, String)>, PembantuError> {
    Ok(sqlx::query_as(
        "SELECT id, content FROM memories WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

pub async fn delete_memory(pool: &PgPool, user_id: i64, id: i64) -> Result<bool, PembantuError> {
    let result = sqlx::query("DELETE FROM memories WHERE user_id = $1 AND id = $2")
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn clear_memories(pool: &PgPool, user_id: i64) -> Result<u64, PembantuError> {
    let result = sqlx::query("DELETE FROM memories WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
