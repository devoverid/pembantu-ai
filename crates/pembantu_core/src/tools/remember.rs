use async_openai::types::chat::ChatCompletionTools;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{db, error::PembantuError};

pub const NAME: &str = "remember";
const MAX_MEMORIES: i64 = 200;

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Save a lasting fact or preference about the current user (e.g. their job, likes, family) so it is known in future conversations.",
        json!({
            "type": "object",
            "properties": {"fact": {"type": "string", "description": "The fact, written as a short standalone sentence."}},
            "required": ["fact"]
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    fact: String,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
    let Args { fact } = parse_args(args)?;
    let fact = fact.trim();
    if fact.is_empty() {
        return Err(PembantuError::Generate("fact must not be empty".into()));
    }
    if db::count_memories(ctx.db, ctx.user.id).await? >= MAX_MEMORIES {
        return Err(PembantuError::Generate("memory full, ask the user to forget something".into()));
    }
    let id = db::add_memory(ctx.db, ctx.user.id, fact).await?;
    Ok(json!({"id": id}).to_string())
}
