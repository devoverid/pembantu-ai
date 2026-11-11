use async_openai::types::chat::ChatCompletionTools;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{error::SitiError, knowledge, truncate_chars};

pub const NAME: &str = "knowledge_search";

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Search the knowledge base for documents relevant to a question. Use this when the user asks about information that may be stored in the app's knowledge base.",
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query or keywords."},
                "limit": {"type": "integer", "description": "Max results (default 5)."}
            },
            "required": ["query"]
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    query: String,
    limit: Option<i64>,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, SitiError> {
    let Args { query, limit } = parse_args(args)?;
    let limit = limit.unwrap_or(5).clamp(1, 10);
    // scope to current user when available (Telegram/API mapped user id)
    let docs = knowledge::search_documents(ctx.db, &query, Some(ctx.user.id), limit).await?;
    if docs.is_empty() {
        // fallback: global/unscoped search (e.g. shared knowledge)
        let docs = knowledge::search_documents(ctx.db, &query, None, limit).await?;
        return Ok(json!({
            "results": docs.iter().map(|d| json!({
                "id": d.id,
                "title": d.title,
                "source": d.source,
                "excerpt": truncate_chars(&d.content, 800),
            })).collect::<Vec<_>>()
        }).to_string());
    }
    Ok(json!({
        "results": docs.iter().map(|d| json!({
            "id": d.id,
            "title": d.title,
            "source": d.source,
            "excerpt": truncate_chars(&d.content, 800),
        })).collect::<Vec<_>>()
    }).to_string())
}
