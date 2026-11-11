use async_openai::types::chat::ChatCompletionTools;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{agent::Attachment, error::SitiError, provider::ImageProvider};

pub const NAME: &str = "generate_image";

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Generate an image from a detailed description. The image is sent to the chat after your reply.",
        json!({
            "type": "object",
            "properties": {"prompt": {"type": "string", "description": "Detailed description of the image."}},
            "required": ["prompt"]
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    prompt: String,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>, image: &ImageProvider) -> Result<String, SitiError> {
    let Args { prompt } = parse_args(args)?;
    let bytes = image.generate_image(&prompt).await?;
    ctx.attachments.push(Attachment::Image { bytes, caption: prompt });
    Ok("Image generated; it will be sent to the chat after your reply.".into())
}
