//! Background worker that runs due reminders and scheduled agent tasks.

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use pembantu_core::{
    agent::{AgentRequest, ChatInfo, ChatKind, Trigger},
    db::{self, NewMessage, TaskRow},
    schedule::Schedule,
};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{MessageId, ParseMode, ThreadId, UserId},
    utils::html,
};

use crate::{
    app::App,
    i18n,
    reply::{self, ReplyTarget},
};

/// Due tasks fire at most this late; the claim query uses a partial index, so polling is cheap.
const TICK: Duration = Duration::from_secs(5);
const BATCH: i64 = 20;

pub async fn run(app: Arc<App>) {
    let mut interval = tokio::time::interval(TICK);
    loop {
        interval.tick().await;
        match db::claim_due_tasks(&app.db, Utc::now(), BATCH).await {
            Ok(tasks) => {
                for task in tasks {
                    tokio::spawn(execute(app.clone(), task));
                }
            }
            Err(e) => log::error!("failed to claim due tasks: {e}"),
        }
    }
}

async fn execute(app: Arc<App>, task: TaskRow) {
    log::info!("running scheduled task #{} ({}) in chat {}", task.id, task.kind, task.chat_id);
    let result = match task.kind.as_str() {
        "reminder" => send_reminder(&app, &task).await,
        "agent_task" => run_agent_task(&app, &task).await,
        other => Err(format!("unknown task kind {other:?}")),
    };
    if let Err(error) = result {
        log::error!("scheduled task #{} failed: {error}", task.id);
        let once = matches!(task.schedule.0, Schedule::Once { .. });
        if let Err(e) = db::record_task_error(&app.db, task.id, &error, once).await {
            log::error!("failed to record error of task #{}: {e}", task.id);
        }
    }
}

fn thread(task: &TaskRow) -> Option<ThreadId> {
    task.thread_id.map(|t| ThreadId(MessageId(t)))
}

async fn send_reminder(app: &App, task: &TaskRow) -> Result<(), String> {
    let (kind, _) = db::get_chat(&app.db, task.chat_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("chat not found")?;
    let text = if kind == "private" {
        format!("⏰ {}", html::escape(&task.payload))
    } else {
        let name = db::get_user(&app.db, task.created_by)
            .await
            .map_err(|e| e.to_string())?
            .map(|u| u.full_name)
            .unwrap_or_else(|| "Reminder".into());
        let mention = html::user_mention(UserId(task.created_by as u64), &name);
        format!("⏰ {mention}: {}", html::escape(&task.payload))
    };

    let mut req = app.bot.send_message(ChatId(task.chat_id), text).parse_mode(ParseMode::Html);
    if let Some(thread_id) = thread(task) {
        req = req.message_thread_id(thread_id);
    }
    let sent = req.await.map_err(|e| e.to_string())?;

    let stored = format!("⏰ {}", task.payload);
    let row = NewMessage {
        chat_id: task.chat_id,
        telegram_message_id: sent.id.0,
        thread_id: task.thread_id,
        user_id: None,
        role: "assistant",
        text: &stored,
        reply_to_telegram_message_id: None,
        created_at: sent.date,
    };
    if let Err(e) = db::insert_message(&app.db, &row).await {
        log::error!("failed to store reminder of task #{}: {e}", task.id);
    }
    Ok(())
}

async fn run_agent_task(app: &Arc<App>, task: &TaskRow) -> Result<(), String> {
    let (kind, title) = db::get_chat(&app.db, task.chat_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("chat not found")?;
    let user = db::get_user(&app.db, task.created_by)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("task creator not found")?;
    let lang = i18n::lang(user.language_code.as_deref());
    let request = AgentRequest {
        chat: ChatInfo {
            id: task.chat_id,
            thread_id: task.thread_id,
            kind: if kind == "private" { ChatKind::Private } else { ChatKind::Group },
            title,
        },
        user,
        text: task.payload.clone(),
        images: Vec::new(),
        reply_context: None,
        trigger: Trigger::ScheduledTask { task_id: task.id },
    };
    let target = ReplyTarget { chat_id: ChatId(task.chat_id), thread_id: thread(task), reply_to: None };
    match reply::respond(app.clone(), request, target, lang).await {
        Some(_) => Ok(()),
        None => Err("the agent run did not produce a reply".into()),
    }
}
