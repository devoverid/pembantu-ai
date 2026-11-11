use async_openai::types::chat::ChatCompletionTools;
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{db, error::SitiError};

pub const NAME: &str = "set_timezone";

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Set the current user's timezone, used for all times they mention.",
        json!({
            "type": "object",
            "properties": {"timezone": {"type": "string", "description": "IANA timezone name, e.g. Asia/Jakarta."}},
            "required": ["timezone"]
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    timezone: String,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, SitiError> {
    let Args { timezone } = parse_args(args)?;
    let tz: Tz = timezone.trim().parse().map_err(|_| {
        SitiError::Generate(format!("unknown timezone {timezone:?}; use an IANA name such as Asia/Jakarta"))
    })?;
    db::set_user_timezone(ctx.db, ctx.user.id, tz.name()).await?;
    Ok(json!({"timezone": tz.name()}).to_string())
}
