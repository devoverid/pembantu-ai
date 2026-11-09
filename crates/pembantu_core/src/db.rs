//! Postgres persistence. Queries are runtime-checked (no `query!` macros) so builds need no database.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, types::Json};
pub use sqlx::PgPool;

use crate::{agent::UserInfo, error::PembantuError, schedule::Schedule};

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

pub struct NewTask<'a> {
    pub chat_id: i64,
    pub thread_id: Option<i32>,
    pub created_by: i64,
    pub kind: &'a str,
    pub label: Option<&'a str>,
    pub payload: &'a str,
    pub schedule: &'a Schedule,
    pub timezone: &'a str,
    pub next_run_at: DateTime<Utc>,
}

pub async fn create_task(pool: &PgPool, t: &NewTask<'_>) -> Result<i64, PembantuError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO scheduled_tasks (chat_id, thread_id, created_by, kind, label, payload, schedule, timezone, next_run_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(t.chat_id)
    .bind(t.thread_id)
    .bind(t.created_by)
    .bind(t.kind)
    .bind(t.label)
    .bind(t.payload)
    .bind(Json(t.schedule))
    .bind(t.timezone)
    .bind(t.next_run_at)
    .fetch_one(pool)
    .await?)
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub chat_id: i64,
    pub thread_id: Option<i32>,
    pub created_by: i64,
    pub kind: String,
    pub label: Option<String>,
    pub payload: String,
    pub schedule: Json<Schedule>,
    pub timezone: String,
    pub next_run_at: Option<DateTime<Utc>>,
    pub status: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A task row with the schedule still undecoded, so one bad row can't fail a whole query.
#[derive(sqlx::FromRow)]
struct RawTaskRow {
    id: i64,
    chat_id: i64,
    thread_id: Option<i32>,
    created_by: i64,
    kind: String,
    label: Option<String>,
    payload: String,
    schedule: Json<Value>,
    timezone: String,
    next_run_at: Option<DateTime<Utc>>,
    status: String,
    last_run_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
}

impl RawTaskRow {
    fn decode(self) -> Result<TaskRow, (i64, String)> {
        let schedule: Schedule =
            serde_json::from_value(self.schedule.0).map_err(|e| (self.id, format!("invalid schedule: {e}")))?;
        Ok(TaskRow {
            id: self.id,
            chat_id: self.chat_id,
            thread_id: self.thread_id,
            created_by: self.created_by,
            kind: self.kind,
            label: self.label,
            payload: self.payload,
            schedule: Json(schedule),
            timezone: self.timezone,
            next_run_at: self.next_run_at,
            status: self.status,
            last_run_at: self.last_run_at,
            last_error: self.last_error,
            created_at: self.created_at,
        })
    }
}

/// Active tasks of a chat, soonest first.
pub async fn list_active_tasks(pool: &PgPool, chat_id: i64) -> Result<Vec<TaskRow>, PembantuError> {
    let rows: Vec<RawTaskRow> = sqlx::query_as(
        "SELECT * FROM scheduled_tasks WHERE chat_id = $1 AND status = 'active' ORDER BY next_run_at, id",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().filter_map(|r| r.decode().ok()).collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelResult {
    Cancelled,
    NotFound,
    NotOwner,
}

/// Cancels an active task of this chat; only its creator may cancel it.
pub async fn cancel_task(pool: &PgPool, chat_id: i64, user_id: i64, id: i64) -> Result<CancelResult, PembantuError> {
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT created_by FROM scheduled_tasks WHERE id = $1 AND chat_id = $2 AND status = 'active'",
    )
    .bind(id)
    .bind(chat_id)
    .fetch_optional(pool)
    .await?;
    match owner {
        None => Ok(CancelResult::NotFound),
        Some(owner) if owner != user_id => Ok(CancelResult::NotOwner),
        Some(_) => {
            sqlx::query("UPDATE scheduled_tasks SET status = 'cancelled' WHERE id = $1 AND status = 'active'")
                .bind(id)
                .execute(pool)
                .await?;
            Ok(CancelResult::Cancelled)
        }
    }
}

pub async fn cancel_labeled_tasks(pool: &PgPool, chat_id: i64, user_id: i64, label: &str) -> Result<u64, PembantuError> {
    let result = sqlx::query(
        "UPDATE scheduled_tasks SET status = 'cancelled'
         WHERE chat_id = $1 AND created_by = $2 AND label = $3 AND status = 'active'",
    )
    .bind(chat_id)
    .bind(user_id)
    .bind(label)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn record_task_error(pool: &PgPool, id: i64, error: &str, mark_failed: bool) -> Result<(), PembantuError> {
    sqlx::query(
        "UPDATE scheduled_tasks SET last_error = $2, status = CASE WHEN $3 THEN 'failed' ELSE status END WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .bind(mark_failed)
    .execute(pool)
    .await?;
    Ok(())
}

/// Claims due tasks at most once: each claimed row is advanced to its next run (or marked done)
/// in the same transaction, before the caller executes it. Missed runs after downtime fire once
/// and then continue from `now`. Rows with an unparsable schedule or timezone are marked failed.
pub async fn claim_due_tasks(pool: &PgPool, now: DateTime<Utc>, limit: i64) -> Result<Vec<TaskRow>, PembantuError> {
    let mut tx = pool.begin().await?;
    let rows: Vec<RawTaskRow> = sqlx::query_as(
        "SELECT * FROM scheduled_tasks WHERE status = 'active' AND next_run_at <= $1
         ORDER BY next_run_at LIMIT $2 FOR UPDATE SKIP LOCKED",
    )
    .bind(now)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;

    let mut claimed = Vec::with_capacity(rows.len());
    for raw in rows {
        let decoded = raw.decode().and_then(|task| match task.timezone.parse::<Tz>() {
            Ok(tz) => Ok((task, tz)),
            Err(_) => Err((task.id, format!("invalid timezone {:?}", task.timezone))),
        });
        let (mut task, tz) = match decoded {
            Ok(ok) => ok,
            Err((id, error)) => {
                sqlx::query("UPDATE scheduled_tasks SET status = 'failed', last_error = $2 WHERE id = $1")
                    .bind(id)
                    .bind(&error)
                    .execute(&mut *tx)
                    .await?;
                continue;
            }
        };
        let next = match &task.schedule.0 {
            Schedule::Daily { .. } => task.schedule.0.next_run_after(tz, now),
            Schedule::Once { .. } => None,
        };
        let status = if next.is_some() { "active" } else { "done" };
        sqlx::query("UPDATE scheduled_tasks SET last_run_at = $2, next_run_at = $3, status = $4 WHERE id = $1")
            .bind(task.id)
            .bind(now)
            .bind(next)
            .bind(status)
            .execute(&mut *tx)
            .await?;
        task.last_run_at = Some(now);
        task.next_run_at = next;
        task.status = status.to_owned();
        claimed.push(task);
    }
    tx.commit().await?;
    Ok(claimed)
}
