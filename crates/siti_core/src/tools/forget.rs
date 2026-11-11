use async_openai::types::chat::ChatCompletionTools;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{db, error::SitiError};

pub const NAME: &str = "forget";

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Delete one of the current user's saved facts by its id (the number shown as [#id] in the known facts).",
        json!({
            "type": "object",
            "properties": {"memory_id": {"type": "integer"}},
            "required": ["memory_id"]
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    memory_id: i64,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, SitiError> {
    let Args { memory_id } = parse_args(args)?;
    let deleted = db::delete_memory(ctx.db, ctx.user.id, memory_id).await?;
    Ok(json!({"deleted": deleted}).to_string())
}
