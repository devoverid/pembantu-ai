//! Built-in and extensible tools the agent can call.
//!
//! Every built-in tool module exposes `NAME`, `definition()` and `run()`.
//! `Tools` acts as a registry: it wires built-ins plus optional dynamic tools so the
//! agent can be extended for different chatbot/agent applications.

pub mod forget;
pub mod generate_image;
pub mod knowledge_search;
pub mod read_chat_history;
pub mod remember;
pub mod scheduling;
pub mod set_timezone;
pub mod web;

use scheduling::{cancel_scheduled_task, create_reminder, list_scheduled_tasks, schedule_agent_task};
use web::{read_webpage, web_search, TavilyClient};

use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObject};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use crate::{
    agent::{Attachment, ChatInfo, ImageInput, Trigger, UserInfo},
    db::PgPool,
    error::SitiError,
    provider::ImageProvider,
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
    /// Images attached to the triggering message or the message it replies to.
    pub images: &'a [ImageInput],
    pub attachments: &'a mut Vec<Attachment>,
}

/// A dynamically registered tool (plugin). Implementations should be cheap to clone/share.
pub type DynToolHandler = Arc<
    dyn for<'a> Fn(&'a str, &'a mut ToolContext<'a>) -> Pin<Box<dyn Future<Output = Result<String, SitiError>> + Send + 'a>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct DynamicTool {
    pub name: String,
    pub definition: ChatCompletionTools,
    pub handler: DynToolHandler,
    /// If true, only available on user-message turns (not scheduled runs).
    pub user_only: bool,
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    dynamic: HashMap<String, DynamicTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: DynamicTool) {
        self.dynamic.insert(tool.name.clone(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&DynamicTool> {
        self.dynamic.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DynamicTool> {
        self.dynamic.values()
    }
}

pub struct Tools {
    search: Option<TavilyClient>,
    image: Option<GeminiAPI>,
    registry: ToolRegistry,
    /// If false, knowledge_search is hidden (default true).
    knowledge_enabled: bool,
}

impl Tools {
    pub fn new(search: Option<TavilyClient>, image: Option<GeminiAPI>) -> Self {
        Self {
            search,
            image,
            registry: ToolRegistry::new(),
            knowledge_enabled: true,
        }
    }

    /// Register a dynamic/custom tool (plugin). Enables extending the agent without
    /// modifying core built-ins.
    pub fn register_dynamic(&mut self, tool: DynamicTool) {
        self.registry.register(tool);
    }

    pub fn set_knowledge_enabled(&mut self, enabled: bool) {
        self.knowledge_enabled = enabled;
    }

    pub fn image(&self) -> Option<&ImageProvider> {
        self.image.as_ref()
    }

    /// Names of the tools available for this trigger. Scheduled runs can't create new tasks.
    pub fn names(&self, trigger: &Trigger) -> Vec<String> {
        let mut names = vec![
            remember::NAME.to_owned(),
            forget::NAME.to_owned(),
            read_chat_history::NAME.to_owned(),
            set_timezone::NAME.to_owned(),
            list_scheduled_tasks::NAME.to_owned(),
            cancel_scheduled_task::NAME.to_owned(),
        ];
        if matches!(trigger, Trigger::UserMessage { .. }) {
            names.extend([create_reminder::NAME.to_owned(), schedule_agent_task::NAME.to_owned()]);
        }
        if self.search.is_some() {
            names.extend([web_search::NAME.to_owned(), read_webpage::NAME.to_owned()]);
        }
        if self.image.is_some() {
            names.push(generate_image::NAME.to_owned());
        }
        if self.knowledge_enabled {
            names.push(knowledge_search::NAME.to_owned());
        }
        for tool in self.registry.iter() {
            if tool.user_only && !matches!(trigger, Trigger::UserMessage { .. }) {
                continue;
            }
            if !names.iter().any(|n| n == &tool.name) {
                names.push(tool.name.clone());
            }
        }
        names
    }

    pub fn definitions(&self, trigger: &Trigger) -> Vec<ChatCompletionTools> {
        self.names(trigger)
            .into_iter()
            .filter_map(|n| self.definition_for(&n))
            .collect()
    }

    /// Runs a tool call. Never fails: errors are returned to the model as `{"error": "..."}`.
    pub async fn execute(&self, name: &str, arguments: &str, ctx: &mut ToolContext<'_>) -> String {
        let args = if arguments.trim().is_empty() { "{}" } else { arguments };
        let result = if self.names(ctx.trigger).iter().any(|n| n == name) {
            self.dispatch(name, args, ctx).await
        } else {
            Err(SitiError::Generate(format!("unknown tool: {name}")))
        };
        match result {
            Ok(output) => truncate_chars(&output, MAX_RESULT_CHARS).to_owned(),
            Err(e) => {
                log::warn!("tool {name} failed: {e}");
                json!({"error": e.to_string()}).to_string()
            }
        }
    }

    fn definition_for(&self, name: &str) -> Option<ChatCompletionTools> {
        if let Some(t) = self.registry.get(name) {
            return Some(t.definition.clone());
        }
        definition(name)
    }

    async fn dispatch(&self, name: &str, args: &str, ctx: &mut ToolContext<'_>) -> Result<String, SitiError> {
        if let Some(tool) = self.registry.get(name) {
            return (tool.handler)(args, ctx).await;
        }
        match (name, &self.search) {
            (remember::NAME, _) => remember::run(args, ctx).await,
            (forget::NAME, _) => forget::run(args, ctx).await,
            (read_chat_history::NAME, _) => read_chat_history::run(args, ctx).await,
            (set_timezone::NAME, _) => set_timezone::run(args, ctx).await,
            (create_reminder::NAME, _) => create_reminder::run(args, ctx).await,
            (schedule_agent_task::NAME, _) => schedule_agent_task::run(args, ctx).await,
            (list_scheduled_tasks::NAME, _) => list_scheduled_tasks::run(args, ctx).await,
            (cancel_scheduled_task::NAME, _) => cancel_scheduled_task::run(args, ctx).await,
            (knowledge_search::NAME, _) => knowledge_search::run(args, ctx).await,
            (web_search::NAME, Some(client)) => web_search::run(args, client).await,
            (read_webpage::NAME, Some(client)) => read_webpage::run(args, client).await,
            (generate_image::NAME, _) => match &self.image {
                Some(image) => generate_image::run(args, ctx, image).await,
                None => Err(SitiError::Generate("image generation is not configured".into())),
            },
            _ => Err(SitiError::Generate(format!("unknown tool: {name}"))),
        }
    }
}

fn definition(name: &str) -> Option<ChatCompletionTools> {
    Some(match name {
        remember::NAME => remember::definition(),
        forget::NAME => forget::definition(),
        read_chat_history::NAME => read_chat_history::definition(),
        set_timezone::NAME => set_timezone::definition(),
        create_reminder::NAME => create_reminder::definition(),
        schedule_agent_task::NAME => schedule_agent_task::definition(),
        list_scheduled_tasks::NAME => list_scheduled_tasks::definition(),
        cancel_scheduled_task::NAME => cancel_scheduled_task::definition(),
        web_search::NAME => web_search::definition(),
        read_webpage::NAME => read_webpage::definition(),
        generate_image::NAME => generate_image::definition(),
        knowledge_search::NAME => knowledge_search::definition(),
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
pub(crate) fn parse_args<T: DeserializeOwned>(args: &str) -> Result<T, SitiError> {
    serde_json::from_str(args).map_err(|e| SitiError::Generate(format!("invalid arguments: {e}")))
}
