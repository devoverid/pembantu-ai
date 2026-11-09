//! Reminder and scheduled agent task tools.

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use super::ToolContext;
use crate::{
    db::{self, NewTask},
    error::PembantuError,
    schedule::{parse_local_datetime, parse_local_time, Schedule},
};

/// JSON-schema properties shared by the scheduling tools.
fn schedule_properties() -> Value {
    json!({
        "at": {"type": "string", "description": "One-off local time, format YYYY-MM-DD HH:MM."},
        "in_seconds": {"type": "integer", "minimum": 10, "maximum": 31536000, "description": "One-off, this many seconds from now (e.g. 20 for \"in 20 seconds\", 7200 for \"in 2 hours\")."},
        "daily_at": {"type": "string", "description": "Repeat every day at this local time, format HH:MM."},
        "weekdays": {"type": "array", "items": {"type": "integer", "minimum": 1, "maximum": 7}, "description": "With daily_at only: ISO weekdays to run on (1 = Monday … 7 = Sunday). Omit for every day."}
    })
}

#[derive(Deserialize)]
struct ScheduleArgs {
    at: Option<String>,
    in_seconds: Option<i64>,
    daily_at: Option<String>,
    weekdays: Option<Vec<u8>>,
}

fn invalid(msg: &str) -> PembantuError {
    PembantuError::Generate(msg.to_owned())
}

impl ScheduleArgs {
    fn into_schedule(self, ctx: &ToolContext<'_>) -> Result<(Schedule, DateTime<Utc>), PembantuError> {
        let given = [self.at.is_some(), self.in_seconds.is_some(), self.daily_at.is_some()];
        if given.iter().filter(|g| **g).count() != 1 {
            return Err(invalid("give exactly one of at, in_seconds or daily_at"));
        }
        if self.weekdays.is_some() && self.daily_at.is_none() {
            return Err(invalid("weekdays can only be used with daily_at"));
        }
        let schedule = if let Some(at) = self.at {
            let at = parse_local_datetime(&at).ok_or_else(|| invalid("at must be formatted YYYY-MM-DD HH:MM"))?;
            Schedule::Once { at }
        } else if let Some(seconds) = self.in_seconds {
            if !(10..=31_536_000).contains(&seconds) {
                return Err(invalid("in_seconds must be between 10 and 31536000"));
            }
            Schedule::Once { at: (ctx.now + Duration::seconds(seconds)).with_timezone(&ctx.timezone).naive_local() }
        } else {
            let daily_at = self.daily_at.unwrap_or_default();
            let time = parse_local_time(&daily_at).ok_or_else(|| invalid("daily_at must be formatted HH:MM"))?;
            let mut weekdays = self.weekdays.unwrap_or_default();
            if weekdays.iter().any(|d| !(1..=7).contains(d)) {
                return Err(invalid("weekdays must be between 1 (Monday) and 7 (Sunday)"));
            }
            weekdays.sort_unstable();
            weekdays.dedup();
            Schedule::Daily { time, weekdays }
        };
        let next = schedule
            .next_run_after(ctx.timezone, ctx.now)
            .ok_or_else(|| invalid("time is in the past or invalid"))?;
        Ok((schedule, next))
    }
}

/// Creates a task for the requester in the current chat and describes it for the model.
async fn create(ctx: &ToolContext<'_>, kind: &str, payload: &str, args: ScheduleArgs) -> Result<String, PembantuError> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(invalid("the reminder text / prompt must not be empty"));
    }
    let (schedule, next) = args.into_schedule(ctx)?;
    let task_id = db::create_task(
        ctx.db,
        &NewTask {
            chat_id: ctx.chat.id,
            thread_id: ctx.chat.thread_id,
            created_by: ctx.user.id,
            kind,
            label: None,
            payload,
            schedule: &schedule,
            timezone: ctx.timezone.name(),
            next_run_at: next,
        },
    )
    .await?;
    Ok(json!({
        "task_id": task_id,
        "next_run_local": next.with_timezone(&ctx.timezone).format("%Y-%m-%d %H:%M:%S").to_string(),
        "schedule": schedule.describe(),
    })
    .to_string())
}

pub mod create_reminder {
    use async_openai::types::chat::ChatCompletionTools;
    use serde::Deserialize;
    use serde_json::json;

    use super::{create, schedule_properties, ScheduleArgs};
    use crate::{
        error::PembantuError,
        tools::{function, parse_args, ToolContext},
    };

    pub const NAME: &str = "create_reminder";

    pub fn definition() -> ChatCompletionTools {
        let mut properties = schedule_properties();
        properties["text"] = json!({"type": "string", "description": "The reminder message to send."});
        function(
            NAME,
            "Send a fixed reminder message in this chat at a time (once or daily).",
            json!({"type": "object", "properties": properties, "required": ["text"]}),
        )
    }

    #[derive(Deserialize)]
    struct Args {
        text: String,
        #[serde(flatten)]
        schedule: ScheduleArgs,
    }

    pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
        let Args { text, schedule } = parse_args(args)?;
        create(ctx, "reminder", &text, schedule).await
    }
}

pub mod schedule_agent_task {
    use async_openai::types::chat::ChatCompletionTools;
    use serde::Deserialize;
    use serde_json::json;

    use super::{create, schedule_properties, ScheduleArgs};
    use crate::{
        error::PembantuError,
        tools::{function, parse_args, ToolContext},
    };

    pub const NAME: &str = "schedule_agent_task";

    pub fn definition() -> ChatCompletionTools {
        let mut properties = schedule_properties();
        properties["prompt"] = json!({"type": "string", "description": "Instruction you will carry out at that time, written as a request from the user."});
        function(
            NAME,
            "Schedule work for you to do later in this chat (once or daily), e.g. a daily briefing or checking news. At run time you get the prompt and your tools.",
            json!({"type": "object", "properties": properties, "required": ["prompt"]}),
        )
    }

    #[derive(Deserialize)]
    struct Args {
        prompt: String,
        #[serde(flatten)]
        schedule: ScheduleArgs,
    }

    pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
        let Args { prompt, schedule } = parse_args(args)?;
        create(ctx, "agent_task", &prompt, schedule).await
    }
}

pub mod list_scheduled_tasks {
    use async_openai::types::chat::ChatCompletionTools;
    use serde_json::json;

    use crate::{
        db,
        error::PembantuError,
        tools::{function, ToolContext},
    };

    pub const NAME: &str = "list_scheduled_tasks";

    pub fn definition() -> ChatCompletionTools {
        function(
            NAME,
            "List the active reminders and scheduled tasks of this chat.",
            json!({"type": "object", "properties": {}}),
        )
    }

    pub async fn run(_args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
        let tasks = db::list_active_tasks(ctx.db, ctx.chat.id).await?;
        let tasks: Vec<_> = tasks
            .iter()
            .map(|t| {
                json!({
                    "task_id": t.id,
                    "kind": t.kind,
                    "payload": t.payload,
                    "schedule": t.schedule.0.describe(),
                    "next_run_local": t.next_run_at.map(|n| n.with_timezone(&ctx.timezone).format("%Y-%m-%d %H:%M").to_string()),
                    "mine": t.created_by == ctx.user.id,
                })
            })
            .collect();
        Ok(json!({"tasks": tasks}).to_string())
    }
}

pub mod cancel_scheduled_task {
    use async_openai::types::chat::ChatCompletionTools;
    use serde::Deserialize;
    use serde_json::json;

    use crate::{
        db::{self, CancelResult},
        error::PembantuError,
        tools::{function, parse_args, ToolContext},
    };

    pub const NAME: &str = "cancel_scheduled_task";

    pub fn definition() -> ChatCompletionTools {
        function(
            NAME,
            "Cancel one of the user's own reminders or scheduled tasks in this chat.",
            json!({"type": "object", "properties": {"task_id": {"type": "integer"}}, "required": ["task_id"]}),
        )
    }

    #[derive(Deserialize)]
    struct Args {
        task_id: i64,
    }

    pub async fn run(args: &str, ctx: &mut ToolContext<'_>) -> Result<String, PembantuError> {
        let Args { task_id } = parse_args(args)?;
        let result = match db::cancel_task(ctx.db, ctx.chat.id, ctx.user.id, task_id).await? {
            CancelResult::Cancelled => "cancelled",
            CancelResult::NotFound => "no active task with this id in this chat",
            CancelResult::NotOwner => "only the user who created this task can cancel it",
        };
        Ok(json!({"result": result}).to_string())
    }
}
