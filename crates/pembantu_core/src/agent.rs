use base64::prelude::*;
use chrono::Utc;
use chrono_tz::Tz;
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    db::{self, PgPool, StoredMessage},
    error::PembantuError,
    llm::{AssistantReply, LlmClient},
    prompt::{detect_language, system_prompt, PromptContext, ReplyLanguage},
    provider::gemini::types::GeminiAPI,
    tools::{web::TavilyClient, ToolContext, Tools},
    truncate_chars,
};

const HISTORY_LIMIT: i64 = 30;
const HISTORY_TEXT_CHARS: usize = 2000;
const MEMORY_LIMIT: i64 = 50;
const MAX_STEPS: usize = 8;

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
    db: PgPool,
    llm: LlmClient,
    tools: Tools,
    default_timezone: chrono_tz::Tz,
}

impl Agent {
    pub fn new(
        db: PgPool,
        llm: LlmClient,
        search: Option<TavilyClient>,
        image: Option<GeminiAPI>,
        default_timezone: chrono_tz::Tz,
    ) -> Self {
        Self { db, llm, tools: Tools::new(search, image), default_timezone }
    }

    pub async fn run(
        &self,
        req: AgentRequest,
        events: UnboundedSender<AgentEvent>,
    ) -> Result<AgentOutput, PembantuError> {
        db::upsert_user(&self.db, &req.user).await?;
        let timezone = self.user_timezone(req.user.id).await?;
        let exclude = match req.trigger {
            Trigger::UserMessage { telegram_message_id } => Some(telegram_message_id),
            Trigger::ScheduledTask { .. } => None,
        };
        let history = db::recent_messages(&self.db, req.chat.id, req.chat.thread_id, exclude, HISTORY_LIMIT).await?;
        // Only the requester's memories, also in groups, so nobody else's facts leak.
        let memories = db::list_memories(&self.db, req.user.id, MEMORY_LIMIT).await?;
        let tool_names = self.tools.names(&req.trigger);
        let definitions = self.tools.definitions(&req.trigger);

        // Scheduled prompts are written in English by the bot, so they use the creator's language.
        let language = match req.trigger {
            Trigger::UserMessage { .. } => detect_language(&req.text),
            Trigger::ScheduledTask { .. } => None,
        }
        .unwrap_or_else(|| ReplyLanguage::from_code(req.user.language_code.as_deref()));
        let system = system_prompt(&PromptContext {
            now_local: Utc::now().with_timezone(&timezone),
            timezone,
            chat: &req.chat,
            user: &req.user,
            memories: &memories,
            tool_names: &tool_names,
            language,
        });
        let mut messages = Vec::with_capacity(history.len() + 2);
        messages.push(json!({"role": "system", "content": system}));
        messages.extend(history.iter().map(history_message));
        messages.push(user_message(&req));

        let mut attachments = Vec::new();
        let mut final_text = None;
        for _ in 0..MAX_STEPS {
            let AssistantReply { raw_message, content, tool_calls } =
                self.llm.complete(&messages, &definitions).await?;
            messages.push(raw_message);
            if tool_calls.is_empty() {
                final_text = Some(content);
                break;
            }
            for call in tool_calls {
                let _ = events.send(AgentEvent::ToolStarted { name: call.name.clone() });
                let mut ctx = ToolContext {
                    db: &self.db,
                    chat: &req.chat,
                    user: &req.user,
                    timezone,
                    now: Utc::now(),
                    trigger: &req.trigger,
                    attachments: &mut attachments,
                };
                let result = self.tools.execute(&call.name, &call.arguments, &mut ctx).await;
                messages.push(json!({"role": "tool", "tool_call_id": call.id, "content": result}));
            }
        }

        let text = match final_text {
            None => "I couldn't finish this within the step limit.".to_owned(),
            // An image-only answer needs no text; reply delivery handles the empty case.
            Some(text) if text.trim().is_empty() && !attachments.is_empty() => String::new(),
            Some(text) if text.trim().is_empty() => "…".to_owned(),
            Some(text) => text,
        };
        Ok(AgentOutput { text, attachments })
    }

    /// The user's saved timezone, or the default when unset or invalid.
    pub async fn user_timezone(&self, user_id: i64) -> Result<Tz, PembantuError> {
        Ok(db::user_timezone(&self.db, user_id)
            .await?
            .and_then(|tz| tz.parse().ok())
            .unwrap_or(self.default_timezone))
    }

    pub async fn generate_image(&self, prompt: &str) -> Result<Vec<u8>, PembantuError> {
        match self.tools.image() {
            Some(image) => image.generate_image(prompt).await,
            None => Err(PembantuError::Config("image generation is not configured".into())),
        }
    }
}

fn history_message(m: &StoredMessage) -> Value {
    let text = truncate_chars(&m.text, HISTORY_TEXT_CHARS);
    if m.role == "assistant" {
        json!({"role": "assistant", "content": text})
    } else {
        let name = m.full_name.as_deref().unwrap_or("Unknown");
        json!({"role": "user", "content": format!("{name}: {text}")})
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
