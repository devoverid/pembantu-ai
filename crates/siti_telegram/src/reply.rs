use std::{sync::Arc, time::Duration};

use chrono::Utc;
use siti_core::{
    agent::{AgentEvent, AgentRequest, Attachment},
    db::{self, NewMessage},
    truncate_chars,
};
use teloxide::{
    payloads::{EditMessageTextSetters, SendMessageSetters, SendPhotoSetters},
    prelude::*,
    types::{ChatAction, InputFile, MessageId, ParseMode, ReplyParameters, ThreadId},
    ApiError, RequestError,
};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    app::App,
    i18n::{self, Lang},
    render::{markdown_to_telegram_html, split_markdown, MAX_CHUNK_CHARS},
};

#[derive(Clone, Copy, Debug)]
pub struct ReplyTarget {
    pub chat_id: ChatId,
    pub thread_id: Option<ThreadId>,
    pub reply_to: Option<MessageId>,
}

impl ReplyTarget {
    /// Reply to `msg` in its chat (and forum topic, if any).
    pub fn to_message(msg: &Message) -> Self {
        Self {
            chat_id: msg.chat.id,
            thread_id: topic_thread(msg),
            reply_to: Some(msg.id),
        }
    }
}

/// The forum topic a message belongs to, if any.
pub fn topic_thread(msg: &Message) -> Option<ThreadId> {
    msg.thread_id.filter(|_| msg.is_topic_message)
}

/// Runs the agent for `req` and delivers the answer: typing indicator, a placeholder that is
/// edited with tool status and then with the final (HTML-rendered, chunked) text, and any
/// attachments. Returns the id of the first message that carries the answer.
pub async fn respond(app: Arc<App>, req: AgentRequest, target: ReplyTarget, lang: Lang) -> Option<MessageId> {
    let typing = spawn_typing(app.bot.clone(), target);
    let result = deliver(&app, req, target, lang).await;
    typing.abort();
    let (id, text) = match result {
        Ok(Some(sent)) => sent,
        Ok(None) => return None,
        Err(e) => {
            log::error!("failed to deliver reply in chat {}: {e}", target.chat_id);
            return None;
        }
    };
    let row = NewMessage {
        chat_id: target.chat_id.0,
        telegram_message_id: id.0,
        thread_id: target.thread_id.map(|t| t.0 .0),
        user_id: None,
        role: "assistant",
        text: &text,
        reply_to_telegram_message_id: target.reply_to.map(|m| m.0),
        created_at: Utc::now(),
    };
    if let Err(e) = db::insert_message(&app.db, &row).await {
        log::error!("failed to store reply in chat {}: {e}", target.chat_id);
    }
    Some(id)
}

/// Returns the first message carrying the answer and the text to remember it by.
async fn deliver(
    app: &App,
    req: AgentRequest,
    target: ReplyTarget,
    lang: Lang,
) -> Result<Option<(MessageId, String)>, RequestError> {
    let placeholder = send_text(app, target, i18n::thinking(lang), false).await?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let status = {
        let bot = app.bot.clone();
        let chat_id = target.chat_id;
        let id = placeholder.id;
        tokio::spawn(async move {
            while let Some(AgentEvent::ToolStarted { name }) = rx.recv().await {
                match bot.edit_message_text(chat_id, id, i18n::tool_status(lang, &name)).await {
                    Ok(_) | Err(RequestError::Api(ApiError::MessageNotModified)) => {}
                    Err(e) => log::warn!("failed to update status message: {e}"),
                }
            }
        })
    };
    // `tx` is moved into `run` and dropped when it returns, which ends the status task; waiting
    // for it guarantees no status edit lands after the final answer.
    let result = app.agent.run(req, tx).await;
    let _ = status.await;

    let output = match result {
        Ok(output) => output,
        Err(e) => {
            log::error!("agent run failed: {e}");
            app.bot.edit_message_text(target.chat_id, placeholder.id, i18n::error(lang)).await?;
            return Ok(None);
        }
    };

    let chunks = split_markdown(&output.text, MAX_CHUNK_CHARS);
    let mut first_id = None;
    for (i, chunk) in chunks.iter().enumerate() {
        if i == 0 {
            edit_markdown(app, target.chat_id, placeholder.id, chunk).await?;
            first_id = Some(placeholder.id);
        } else {
            send_markdown(app, target, chunk).await?;
        }
    }

    let stored_text = if !chunks.is_empty() {
        output.text
    } else if let Some(Attachment::Image { caption, .. }) = output.attachments.first() {
        format!("[image] {caption}")
    } else {
        "…".to_owned()
    };
    let has_attachments = !output.attachments.is_empty();
    for attachment in output.attachments {
        match attachment {
            Attachment::Image { bytes, caption } => {
                let mut req = app
                    .bot
                    .send_photo(target.chat_id, InputFile::memory(bytes))
                    .caption(truncate_chars(&caption, 1024));
                if let Some(thread_id) = target.thread_id {
                    req = req.message_thread_id(thread_id);
                }
                if let Some(reply_to) = target.reply_to {
                    req = req.reply_parameters(ReplyParameters::new(reply_to).allow_sending_without_reply());
                }
                let sent = req.await?;
                first_id.get_or_insert(sent.id);
            }
        }
    }

    if chunks.is_empty() {
        if has_attachments {
            app.bot.delete_message(target.chat_id, placeholder.id).await?;
        } else {
            app.bot.edit_message_text(target.chat_id, placeholder.id, "…").await?;
            first_id = Some(placeholder.id);
        }
    }
    Ok(first_id.map(|id| (id, stored_text)))
}

fn spawn_typing(bot: Bot, target: ReplyTarget) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let mut req = bot.send_chat_action(target.chat_id, ChatAction::Typing);
            if let Some(thread_id) = target.thread_id {
                req = req.message_thread_id(thread_id);
            }
            if let Err(e) = req.await {
                log::debug!("failed to send typing action: {e}");
            }
            tokio::time::sleep(Duration::from_secs(4)).await;
        }
    })
}

/// Sends `text` to the target, as Telegram HTML when `html` is set.
pub async fn send_text(app: &App, target: ReplyTarget, text: &str, html: bool) -> Result<Message, RequestError> {
    let mut req = app.bot.send_message(target.chat_id, text);
    if html {
        req = req.parse_mode(ParseMode::Html);
    }
    if let Some(thread_id) = target.thread_id {
        req = req.message_thread_id(thread_id);
    }
    if let Some(reply_to) = target.reply_to {
        req = req.reply_parameters(ReplyParameters::new(reply_to).allow_sending_without_reply());
    }
    req.await
}

/// Telegram rejects HTML it can't parse, and rendered HTML can outgrow the length limit; in both
/// cases the raw Markdown is sent instead.
fn needs_plain_fallback(e: &RequestError) -> bool {
    matches!(
        e,
        RequestError::Api(
            ApiError::CantParseEntities(_)
                | ApiError::MessageIsTooLong
                | ApiError::EditedMessageIsTooLong
                | ApiError::MessageTextIsEmpty
        )
    )
}

async fn send_markdown(app: &App, target: ReplyTarget, md: &str) -> Result<Message, RequestError> {
    match send_text(app, target, &markdown_to_telegram_html(md), true).await {
        Err(e) if needs_plain_fallback(&e) => send_text(app, target, md, false).await,
        other => other,
    }
}

async fn edit_markdown(app: &App, chat_id: ChatId, id: MessageId, md: &str) -> Result<(), RequestError> {
    let html = app
        .bot
        .edit_message_text(chat_id, id, markdown_to_telegram_html(md))
        .parse_mode(ParseMode::Html)
        .await;
    match html {
        Ok(_) | Err(RequestError::Api(ApiError::MessageNotModified)) => Ok(()),
        Err(e) if needs_plain_fallback(&e) => app.bot.edit_message_text(chat_id, id, md).await.map(drop),
        Err(e) => Err(e),
    }
}
