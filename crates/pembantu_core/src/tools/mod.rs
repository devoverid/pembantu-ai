//! Built-in tools the agent can call. Every tool module exposes `NAME`, `definition()` and `run()`.

pub mod forget;
pub mod read_chat_history;
pub mod remember;
pub mod set_timezone;
pub mod web;

use web::{read_webpage, web_search, TavilyClient};

use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObject};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::{
    agent::{Attachment, ChatInfo, Trigger, UserInfo},
    db::PgPool,
    error::PembantuError,
    provider::gemini::types::GeminiAPI,
    truncate_chars,
};

const MAX_RESULT_CHARS: usize = 20_000;

pub struct ToolContext<'a> {
    pub db: &'a PgPool,
    pub chat: &'a ChatInfo,
    pub user: &'a UserInfo,
    pub timezone: Tz,
    pub now: DateTime<Utc>,
    pub trigger: &'a Trigger,
    pub attachments: &'a mut Vec<Attachment>,
}

pub struct Tools {
    search: Option<TavilyClient>,
    image: Option<GeminiAPI>,
}

impl Tools {
    pub fn new(search: Option<TavilyClient>, image: Option<GeminiAPI>) -> Self {
        Self { search, image }
    }

    pub fn image(&self) -> Option<&GeminiAPI> {
        self.image.as_ref()
    }

    /// Names of the tools available for this trigger.
    pub fn names(&self, _trigger: &Trigger) -> Vec<&'static str> {
        let mut names = vec![remember::NAME, forget::NAME, read_chat_history::NAME, set_timezone::NAME];
        if self.search.is_some() {
            names.extend([web_search::NAME, read_webpage::NAME]);
        }
        names
    }

    pub fn definitions(&self, trigger: &Trigger) -> Vec<ChatCompletionTools> {
        self.names(trigger).into_iter().filter_map(definition).collect()
    }

    /// Runs a tool call. Never fails: errors are returned to the model as `{"error": "..."}`.
    pub async fn execute(&self, name: &str, arguments: &str, ctx: &mut ToolContext<'_>) -> String {
        let args = if arguments.trim().is_empty() { "{}" } else { arguments };
        let result = if self.names(ctx.trigger).contains(&name) {
            self.dispatch(name, args, ctx).await
        } else {
            Err(PembantuError::Generate(format!("unknown tool: {name}")))
        };
        match result {
            Ok(output) => truncate_chars(&output, MAX_RESULT_CHARS).to_owned(),
            Err(e) => {
                log::warn!("tool {name} failed: {e}");
                json!({"error": e.to_string()}).to_string()
            }
        }
    }

    async fn dispatch(&self, name: &str, args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
        match (name, &self.search) {
            (remember::NAME, _) => remember::run(args, ctx).await,
            (forget::NAME, _) => forget::run(args, ctx).await,
            (read_chat_history::NAME, _) => read_chat_history::run(args, ctx).await,
            (set_timezone::NAME, _) => set_timezone::run(args, ctx).await,
            (web_search::NAME, Some(client)) => web_search::run(args, client).await,
            (read_webpage::NAME, Some(client)) => read_webpage::run(args, client).await,
            _ => Err(PembantuError::Generate(format!("unknown tool: {name}"))),
        }
    }
}

fn definition(name: &str) -> Option<ChatCompletionTools> {
    Some(match name {
        remember::NAME => remember::definition(),
        forget::NAME => forget::definition(),
        read_chat_history::NAME => read_chat_history::definition(),
        set_timezone::NAME => set_timezone::definition(),
        web_search::NAME => web_search::definition(),
        read_webpage::NAME => read_webpage::definition(),
        _ => return None,
    })
}

/// Builds a function tool definition.
pub(crate) fn function(name: &str, description: &str, parameters: Value) -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObject {
            name: name.to_owned(),
            description: Some(description.to_owned()),
            parameters: Some(parameters),
            strict: None,
        },
    })
}

/// Parses tool arguments, reporting schema mismatches back to the model.
pub(crate) fn parse_args<T: DeserializeOwned>(args: &str) -> Result<T, PembantuError> {
    serde_json::from_str(args).map_err(|e| PembantuError::Generate(format!("invalid arguments: {e}")))
}
