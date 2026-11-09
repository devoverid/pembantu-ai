use base64::prelude::*;
use chrono::Utc;
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    error::PembantuError,
    llm::LlmClient,
    prompt::{detect_language, system_prompt, PromptContext, ReplyLanguage},
    provider::gemini::types::GeminiAPI,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatKind {
    Private,
    Group,
}

#[derive(Clone, Debug)]
pub struct ChatInfo {
    pub id: i64,
    pub thread_id: Option<i32>,
    pub kind: ChatKind,
    pub title: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UserInfo {
    pub id: i64,
    pub full_name: String,
    pub username: Option<String>,
    pub language_code: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ImageInput {
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum Trigger {
    UserMessage { telegram_message_id: i32 },
    ScheduledTask { task_id: i64 },
}

#[derive(Clone, Debug)]
pub struct AgentRequest {
    pub chat: ChatInfo,
    pub user: UserInfo,
    pub text: String,
    pub images: Vec<ImageInput>,
    pub reply_context: Option<String>,
    pub trigger: Trigger,
}

#[derive(Clone, Debug)]
pub enum Attachment {
    Image { bytes: Vec<u8>, caption: String },
}

#[derive(Clone, Debug, Default)]
pub struct AgentOutput {
    pub text: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug)]
pub enum AgentEvent {
    ToolStarted { name: String },
}

pub struct Agent {
    llm: LlmClient,
    image: Option<GeminiAPI>,
    default_timezone: chrono_tz::Tz,
}

impl Agent {
    pub fn new(llm: LlmClient, image: Option<GeminiAPI>, default_timezone: chrono_tz::Tz) -> Self {
        Self { llm, image, default_timezone }
    }

    pub async fn run(
        &self,
        req: AgentRequest,
        _events: UnboundedSender<AgentEvent>,
    ) -> Result<AgentOutput, PembantuError> {
        let timezone = self.default_timezone;
        let language = detect_language(&req.text)
            .unwrap_or_else(|| ReplyLanguage::from_code(req.user.language_code.as_deref()));
        let system = system_prompt(&PromptContext {
            now_local: Utc::now().with_timezone(&timezone),
            timezone,
            chat: &req.chat,
            user: &req.user,
            memories: &[],
            tool_names: &[],
            language,
        });
        let messages = [json!({"role": "system", "content": system}), user_message(&req)];
        let reply = self.llm.complete(&messages, &[]).await?;
        let text = if reply.content.trim().is_empty() { "…".to_owned() } else { reply.content };
        Ok(AgentOutput { text, attachments: Vec::new() })
    }

    pub async fn generate_image(&self, prompt: &str) -> Result<Vec<u8>, PembantuError> {
        match &self.image {
            Some(image) => image.generate_image(prompt).await,
            None => Err(PembantuError::Config("image generation is not configured".into())),
        }
    }
}

fn user_message(req: &AgentRequest) -> Value {
    let mut text = format!("{}: {}", req.user.full_name, req.text);
    if let Some(reply_context) = &req.reply_context {
        text = format!("(replying to {reply_context})\n{text}");
    }
    if req.images.is_empty() {
        return json!({"role": "user", "content": text});
    }
    let mut parts = vec![json!({"type": "text", "text": text})];
    parts.extend(req.images.iter().map(|image| {
        json!({
            "type": "image_url",
            "image_url": {"url": format!("data:{};base64,{}", image.mime, BASE64_STANDARD.encode(&image.bytes))}
        })
    }));
    json!({"role": "user", "content": parts})
}
