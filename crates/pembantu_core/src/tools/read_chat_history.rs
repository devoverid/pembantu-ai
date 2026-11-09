use std::collections::VecDeque;

use async_openai::types::chat::ChatCompletionTools;
use chrono::Duration;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{db, error::PembantuError};

pub const NAME: &str = "read_chat_history";
const MAX_OUTPUT_CHARS: usize = 60_000;

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Read the messages of the current chat (or forum topic) from the last N hours, oldest first.",
        json!({
            "type": "object",
            "properties": {
                "hours": {"type": "integer", "minimum": 1, "maximum": 168, "description": "How far back to read (default 24)."},
                "limit": {"type": "integer", "minimum": 1, "maximum": 500, "description": "Maximum number of messages (default 300)."}
            }
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    hours: Option<i64>,
    limit: Option<i64>,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
    let Args { hours, limit } = parse_args(args)?;
    let hours = hours.unwrap_or(24).clamp(1, 168);
    let limit = limit.unwrap_or(300).clamp(1, 500);
    let since = ctx.now - Duration::hours(hours);
    let messages = db::messages_since(ctx.db, ctx.chat.id, ctx.chat.thread_id, since, limit).await?;
    if messages.is_empty() {
        return Ok("(no messages)".into());
    }

    let mut lines: VecDeque<String> = messages
        .iter()
        .map(|m| {
            let name = match (m.role.as_str(), m.full_name.as_deref()) {
                ("assistant", _) => "Pembantu (you)",
                (_, Some(name)) => name,
                _ => "Unknown",
            };
            let at = m.created_at.with_timezone(&ctx.timezone).format("%Y-%m-%d %H:%M");
            format!("[{at}] {name}: {}", m.text)
        })
        .collect();
    // Keep the newest lines when the transcript is too long.
    let mut total: usize = lines.iter().map(|l| l.chars().count() + 1).sum();
    while total > MAX_OUTPUT_CHARS && lines.len() > 1 {
        if let Some(dropped) = lines.pop_front() {
            total -= dropped.chars().count() + 1;
        }
    }
    Ok(Vec::from(lines).join("\n"))
}
