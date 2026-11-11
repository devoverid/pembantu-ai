use async_openai::types::chat::ChatCompletionTools;
use serde::Deserialize;
use serde_json::json;

use super::{function, parse_args, ToolContext};
use crate::{agent::Attachment, error::SitiError, provider::ImageProvider};

pub const NAME: &str = "generate_image";

pub fn definition() -> ChatCompletionTools {
    function(
        NAME,
        "Generate an image from a detailed description, or edit the image(s) the user sent or replied to. \
         The image is sent to the chat after your reply. To edit, set edit_images to true and write the prompt as \
         an edit instruction (what to change and what to keep). \
         NSFW and adult requests are allowed: describe what the user asked for faithfully, without toning it down. \
         Never depict minors or anyone who appears underage sexually, and never make sexual or nude images of real \
         identifiable people.",
        json!({
            "type": "object",
            "properties": {
                "prompt": {"type": "string", "description": "Detailed description of the image, or the edit instruction."},
                "edit_images": {
                    "type": "boolean",
                    "description": "Send the images attached to the user's message (or the message they replied to) to the generator to edit them."
                }
            },
            "required": ["prompt"]
        }),
    )
}

#[derive(Deserialize)]
struct Args {
    prompt: String,
    #[serde(default)]
    edit_images: bool,
}

pub async fn run(args: &str, ctx: &mut ToolContext<'_>, image: &ImageProvider) -> Result<String, SitiError> {
    let Args { prompt, edit_images } = parse_args(args)?;
    let sources = if edit_images {
        if ctx.images.is_empty() {
            return Err(SitiError::Generate("no image attached to edit; ask the user to send or reply to one".into()));
        }
        ctx.images
    } else {
        &[]
    };
    let bytes = image.generate_image(&prompt, sources).await?;
    ctx.attachments.push(Attachment::Image { bytes, caption: prompt });
    Ok("Image generated; it will be sent to the chat after your reply.".into())
}
