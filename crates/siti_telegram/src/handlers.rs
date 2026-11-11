use std::sync::Arc;

use chrono::Utc;
use siti_core::{
    agent::{AgentRequest, ChatInfo, ChatKind, ImageInput, Trigger, UserInfo},
    db::{self, NewMessage, NewTask},
    error::SitiError,
    schedule::{parse_local_time, Schedule},
    truncate_chars,
};
use teloxide::{
    net::Download,
    payloads::{SendChatActionSetters, SendPhotoSetters},
    prelude::*,
    types::{ChatAction, InputFile, MessageEntityKind, PhotoSize, ReplyParameters, User},
    utils::command::BotCommands,
};

use crate::{
    app::App,
    command::Command,
    i18n::{self, Lang},
    reply::{self, ReplyTarget},
};

/// Entry point for every message update. Every message is stored first (for chat context);
/// long work is spawned so that teloxide's per-chat sequential dispatch keeps ingesting updates
/// while the agent runs.
pub async fn handle_message(app: Arc<App>, msg: Message) -> ResponseResult<()> {
    if let Err(e) = store_incoming(&app, &msg).await {
        log::error!("failed to store message in chat {}: {e}", msg.chat.id);
    }
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    if from.is_bot {
        return Ok(());
    }
    let text = msg.text().or(msg.caption()).unwrap_or_default().to_owned();

    if let Ok(cmd) = Command::parse(&text, app.me.username()) {
        tokio::spawn(handle_command(app, msg, cmd));
    } else if should_respond(&app, &msg) {
        tokio::spawn(respond_to(app, msg, text));
    }
    Ok(())
}

fn chat_kind(chat: &teloxide::types::Chat) -> &'static str {
    if chat.is_private() {
        "private"
    } else if chat.is_supergroup() {
        "supergroup"
    } else if chat.is_group() {
        "group"
    } else {
        "channel"
    }
}

async fn store_incoming(app: &App, msg: &Message) -> Result<(), siti_core::error::SitiError> {
    let user = msg.from.as_ref().map(user_info);
    if let Some(user) = &user {
        db::upsert_user(&app.db, user).await?;
    }
    db::upsert_chat(&app.db, msg.chat.id.0, chat_kind(&msg.chat), msg.chat.title()).await?;
    let Some(text) = describe(msg) else {
        return Ok(());
    };
    let row = NewMessage {
        chat_id: msg.chat.id.0,
        telegram_message_id: msg.id.0,
        thread_id: reply::topic_thread(msg).map(|t| t.0 .0),
        user_id: user.map(|u| u.id),
        role: "user",
        text: &text,
        reply_to_telegram_message_id: msg.reply_to_message().map(|r| r.id.0),
        created_at: msg.date,
    };
    db::insert_message(&app.db, &row).await
}

/// A text rendering of a message for chat history, or `None` for kinds not worth storing.
fn describe(msg: &Message) -> Option<String> {
    let caption = msg.caption().unwrap_or_default();
    let text = if let Some(text) = msg.text() {
        text.to_owned()
    } else if msg.photo().is_some() {
        format!("[photo] {caption}")
    } else if msg.video().is_some() {
        format!("[video] {caption}")
    } else if let Some(doc) = msg.document() {
        format!("[document: {}] {caption}", doc.file_name.as_deref().unwrap_or("file"))
    } else if msg.voice().is_some() {
        "[voice message]".to_owned()
    } else {
        format!("[sticker {}]", msg.sticker()?.emoji.as_deref().unwrap_or_default())
    };
    Some(text.trim().to_owned())
}

fn should_respond(app: &App, msg: &Message) -> bool {
    if msg.chat.is_private() {
        return msg.text().is_some() || msg.photo().is_some();
    }
    if !(msg.chat.is_group() || msg.chat.is_supergroup()) {
        return false;
    }
    let replied_to_bot = msg
        .reply_to_message()
        .and_then(|r| r.from.as_ref())
        .is_some_and(|u| u.id == app.me.id);
    if replied_to_bot {
        return true;
    }
    let mention = format!("@{}", app.me.username());
    let mentioned = msg
        .parse_entities()
        .or_else(|| msg.parse_caption_entities())
        .unwrap_or_default()
        .iter()
        .any(|e| matches!(e.kind(), MessageEntityKind::Mention) && e.text().eq_ignore_ascii_case(&mention));
    mentioned
        || msg
            .text()
            .or(msg.caption())
            .and_then(|t| t.get(..5))
            .is_some_and(|p| p.eq_ignore_ascii_case("Siti,"))
}

fn user_lang(msg: &Message) -> Lang {
    i18n::lang(msg.from.as_ref().and_then(|u| u.language_code.as_deref()))
}

async fn handle_command(app: Arc<App>, msg: Message, cmd: Command) {
    let lang = user_lang(&msg);
    let target = ReplyTarget::to_message(&msg);
    let result = match cmd {
        Command::Start => reply::send_text(&app, target, i18n::welcome(lang), false).await.map(drop),
        Command::Help => reply::send_text(&app, target, i18n::help(lang), false).await.map(drop),
        Command::Ask(question) => {
            let question = Some(question.trim())
                .filter(|q| !q.is_empty())
                .or_else(|| msg.reply_to_message().and_then(|r| r.text().or(r.caption())))
                .map(str::to_owned);
            match question {
                Some(question) => {
                    respond_to(app, msg, question).await;
                    Ok(())
                }
                None => reply::send_text(&app, target, i18n::usage_ask(lang), false).await.map(drop),
            }
        }
        Command::Image(prompt) => generate_image(&app, &msg, prompt.trim(), lang).await,
        cmd @ (Command::Memory(_)
        | Command::Briefing(_)
        | Command::Tasks(_)
        | Command::Timezone(_)
        | Command::Persona(_)) => {
            let text = text_command(&app, &msg, cmd, lang).await.unwrap_or_else(|e| {
                log::error!("command failed: {e}");
                i18n::error(lang).to_owned()
            });
            reply::send_text(&app, target, truncate_chars(&text, 4000), false).await.map(drop)
        }
        Command::Summary(args) => {
            let args = args.trim();
            let hours = if args.is_empty() { Some(24) } else { args.parse::<u32>().ok().filter(|h| (1..=168).contains(h)) };
            match hours {
                Some(hours) => {
                    let prompt = format!(
                        "Summarize this chat from the last {hours} hours using read_chat_history. \
                         Group by topic, then list decisions and open action items with owners."
                    );
                    respond_to(app, msg, prompt).await;
                    Ok(())
                }
                None => reply::send_text(&app, target, i18n::usage_summary(lang), false).await.map(drop),
            }
        }
    };
    if let Err(e) = result {
        log::error!("failed to handle command in chat {}: {e}", target.chat_id);
    }
}

/// Commands that answer with a plain text message.
async fn text_command(app: &App, msg: &Message, cmd: Command, lang: Lang) -> Result<String, SitiError> {
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).ok_or(SitiError::Generate("message has no sender".into()))?;
    match cmd {
        Command::Memory(args) => memory_command(app, user_id, args.trim(), lang).await,
        Command::Briefing(args) => briefing_command(app, msg, user_id, args.trim(), lang).await,
        Command::Tasks(args) => tasks_command(app, msg.chat.id.0, user_id, args.trim(), lang).await,
        Command::Timezone(args) => timezone_command(app, user_id, args.trim(), lang).await,
        Command::Persona(args) => persona_command(app, msg, args.trim(), lang).await,
        _ => Err(SitiError::Generate("not a text command".into())),
    }
}

const PERSONA_MAX_CHARS: usize = 1500;

/// Shows, sets or resets the chat's persona. Anyone may view it; changing it is limited to private
/// chats and group administrators.
async fn persona_command(app: &App, msg: &Message, args: &str, lang: Lang) -> Result<String, SitiError> {
    let chat_id = msg.chat.id.0;
    if args.is_empty() {
        return Ok(match db::chat_persona(&app.db, chat_id).await? {
            Some(persona) => i18n::persona_current(lang, &persona),
            None => i18n::usage_persona(lang).to_owned(),
        });
    }
    if !can_change_persona(app, msg).await? {
        return Ok(i18n::persona_admin_only(lang).to_owned());
    }
    if args.eq_ignore_ascii_case("reset") || args.eq_ignore_ascii_case("off") {
        db::set_chat_persona(&app.db, chat_id, None).await?;
        return Ok(i18n::persona_reset(lang).to_owned());
    }
    if args.chars().count() > PERSONA_MAX_CHARS {
        return Ok(i18n::persona_too_long(lang, PERSONA_MAX_CHARS));
    }
    db::set_chat_persona(&app.db, chat_id, Some(args)).await?;
    Ok(i18n::persona_set(lang).to_owned())
}

async fn can_change_persona(app: &App, msg: &Message) -> Result<bool, SitiError> {
    if msg.chat.is_private() {
        return Ok(true);
    }
    if !(msg.chat.is_group() || msg.chat.is_supergroup()) {
        return Ok(false);
    }
    let Some(from) = msg.from.as_ref() else {
        return Ok(false);
    };
    let member = app
        .bot
        .get_chat_member(msg.chat.id, from.id)
        .await
        .map_err(|e| SitiError::Generate(format!("getChatMember failed: {e}")))?;
    Ok(member.is_privileged())
}

const BRIEFING_LABEL: &str = "daily_briefing";
const BRIEFING_PROMPT: &str = "Prepare my daily briefing: 1) summarize important messages in this chat from the last 24 hours \
    with read_chat_history; 2) list my upcoming scheduled tasks with list_scheduled_tasks; 3) if web_search is available, \
    give 3 short news headlines matching my known interests. Keep it brief.";

async fn briefing_command(app: &App, msg: &Message, user_id: i64, args: &str, lang: Lang) -> Result<String, SitiError> {
    let chat_id = msg.chat.id.0;
    if args.is_empty() {
        let current = db::list_active_tasks(&app.db, chat_id)
            .await?
            .into_iter()
            .find(|t| t.created_by == user_id && t.label.as_deref() == Some(BRIEFING_LABEL));
        return Ok(match current {
            Some(task) => i18n::briefing_current(lang, &task.schedule.0.describe(), &task.timezone),
            None => i18n::usage_briefing(lang).to_owned(),
        });
    }
    if args.eq_ignore_ascii_case("off") {
        let cancelled = db::cancel_labeled_tasks(&app.db, chat_id, user_id, BRIEFING_LABEL).await?;
        return Ok(i18n::briefing_off(lang, cancelled > 0).to_owned());
    }
    let Some(time) = parse_local_time(args) else {
        return Ok(i18n::usage_briefing(lang).to_owned());
    };
    let tz = app.agent.user_timezone(user_id).await?;
    let schedule = Schedule::Daily { time, weekdays: Vec::new() };
    let next = schedule
        .next_run_after(tz, Utc::now())
        .ok_or(SitiError::Generate("briefing time has no next run".into()))?;
    db::cancel_labeled_tasks(&app.db, chat_id, user_id, BRIEFING_LABEL).await?;
    db::create_task(
        &app.db,
        &NewTask {
            chat_id,
            thread_id: reply::topic_thread(msg).map(|t| t.0 .0),
            created_by: user_id,
            kind: "agent_task",
            label: Some(BRIEFING_LABEL),
            payload: BRIEFING_PROMPT,
            schedule: &schedule,
            timezone: tz.name(),
            next_run_at: next,
        },
    )
    .await?;
    let next_local = next.with_timezone(&tz).format("%Y-%m-%d %H:%M").to_string();
    Ok(i18n::briefing_set(lang, &time.format("%H:%M").to_string(), tz.name(), &next_local))
}

async fn tasks_command(app: &App, chat_id: i64, user_id: i64, args: &str, lang: Lang) -> Result<String, SitiError> {
    let mut words = args.split_whitespace();
    match (words.next(), words.next(), words.next()) {
        (None, _, _) => {
            let tasks = db::list_active_tasks(&app.db, chat_id).await?;
            if tasks.is_empty() {
                return Ok(i18n::tasks_empty(lang).to_owned());
            }
            let mut text = i18n::tasks_header(lang).to_owned();
            for task in tasks {
                let icon = if task.kind == "reminder" { "⏰" } else { "🤖" };
                let payload = if task.label.as_deref() == Some(BRIEFING_LABEL) {
                    i18n::briefing_name(lang)
                } else {
                    truncate_chars(&task.payload, 80)
                };
                let next = match (task.next_run_at, task.timezone.parse::<chrono_tz::Tz>()) {
                    (Some(next), Ok(tz)) => next.with_timezone(&tz).format("%Y-%m-%d %H:%M").to_string(),
                    _ => "-".to_owned(),
                };
                text.push_str(&format!(
                    "\n#{} {icon} {payload} — {} — {next}",
                    task.id,
                    task.schedule.0.describe()
                ));
            }
            Ok(text)
        }
        (Some("cancel"), Some(id), None) => match id.trim_start_matches('#').parse::<i64>() {
            Ok(id) => Ok(i18n::task_cancel_result(lang, db::cancel_task(&app.db, chat_id, user_id, id).await?).to_owned()),
            Err(_) => Ok(i18n::usage_tasks(lang).to_owned()),
        },
        _ => Ok(i18n::usage_tasks(lang).to_owned()),
    }
}

async fn timezone_command(app: &App, user_id: i64, args: &str, lang: Lang) -> Result<String, SitiError> {
    if args.is_empty() {
        let tz = app.agent.user_timezone(user_id).await?;
        return Ok(i18n::timezone_current(lang, tz.name()));
    }
    match args.parse::<chrono_tz::Tz>() {
        Ok(tz) => {
            db::set_user_timezone(&app.db, user_id, tz.name()).await?;
            Ok(i18n::timezone_set(lang, tz.name()))
        }
        Err(_) => Ok(i18n::timezone_invalid(lang).to_owned()),
    }
}

async fn memory_command(app: &App, user_id: i64, args: &str, lang: Lang) -> Result<String, SitiError> {
    let mut words = args.split_whitespace();
    match (words.next(), words.next(), words.next()) {
        (None, _, _) => {
            let memories = db::list_memories(&app.db, user_id, 200).await?;
            if memories.is_empty() {
                return Ok(i18n::memory_empty(lang).to_owned());
            }
            let mut text = i18n::memory_header(lang).to_owned();
            for (id, content) in memories {
                text.push_str(&format!("\n#{id} {content}"));
            }
            Ok(text)
        }
        (Some("clear"), None, _) => Ok(i18n::memory_cleared(lang, db::clear_memories(&app.db, user_id).await?)),
        (Some("forget"), Some(id), None) => match id.trim_start_matches('#').parse::<i64>() {
            Ok(id) if db::delete_memory(&app.db, user_id, id).await? => Ok(i18n::memory_forgotten(lang, id)),
            Ok(id) => Ok(i18n::memory_not_found(lang, id)),
            Err(_) => Ok(i18n::usage_memory(lang).to_owned()),
        },
        _ => Ok(i18n::usage_memory(lang).to_owned()),
    }
}

/// `/image <prompt>`: generates an image, or edits the photo attached to the command or the
/// message it replies to.
async fn generate_image(app: &App, msg: &Message, prompt: &str, lang: Lang) -> ResponseResult<()> {
    let target = ReplyTarget::to_message(msg);
    if prompt.is_empty() {
        reply::send_text(app, target, i18n::usage_image(lang), false).await?;
        return Ok(());
    }
    let mut action = app.bot.send_chat_action(msg.chat.id, ChatAction::UploadPhoto);
    if let Some(thread_id) = target.thread_id {
        action = action.message_thread_id(thread_id);
    }
    if let Err(e) = action.await {
        log::debug!("failed to send upload action: {e}");
    }
    let images = message_images(app, msg).await;
    match app.agent.generate_image(prompt, &images).await {
        Ok(bytes) => {
            let mut req = app
                .bot
                .send_photo(msg.chat.id, InputFile::memory(bytes))
                .caption(truncate_chars(prompt, 1024))
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply());
            if let Some(thread_id) = target.thread_id {
                req = req.message_thread_id(thread_id);
            }
            req.await?;
        }
        Err(e) => {
            log::error!("image generation failed: {e}");
            reply::send_text(app, target, i18n::error(lang), false).await?;
        }
    }
    Ok(())
}

async fn respond_to(app: Arc<App>, msg: Message, text: String) {
    let Some(from) = msg.from.as_ref() else {
        return;
    };
    let lang = user_lang(&msg);
    let request = build_request(&app, &msg, from, text).await;
    reply::respond(app, request, ReplyTarget::to_message(&msg), lang).await;
}

pub fn chat_info(msg: &Message) -> ChatInfo {
    ChatInfo {
        id: msg.chat.id.0,
        thread_id: reply::topic_thread(msg).map(|t| t.0 .0),
        kind: if msg.chat.is_private() { ChatKind::Private } else { ChatKind::Group },
        title: msg.chat.title().map(str::to_owned),
    }
}

pub fn user_info(user: &User) -> UserInfo {
    UserInfo {
        id: user.id.0 as i64,
        full_name: user.full_name(),
        username: user.username.clone(),
        language_code: user.language_code.clone(),
    }
}

async fn build_request(app: &App, msg: &Message, from: &User, text: String) -> AgentRequest {
    let images = message_images(app, msg).await;
    let text = if text.trim().is_empty() && msg.photo().is_some() { "(sent a photo)".to_owned() } else { text };

    let reply_context = msg.reply_to_message().map(|r| {
        let sender = r.from.as_ref().map(|u| u.full_name()).unwrap_or_else(|| "Unknown".into());
        let content = r.text().or(r.caption()).unwrap_or(if r.photo().is_some() { "[photo]" } else { "" });
        truncate_chars(&format!("{sender}: {content}"), 1000).to_owned()
    });

    AgentRequest {
        chat: chat_info(msg),
        user: user_info(from),
        text,
        images,
        reply_context,
        trigger: Trigger::UserMessage { telegram_message_id: msg.id.0 },
    }
}

/// Photos attached to `msg` and to the message it replies to, so the agent can see and edit them.
async fn message_images(app: &App, msg: &Message) -> Vec<ImageInput> {
    let mut images = Vec::new();
    for source in [Some(msg), msg.reply_to_message()].into_iter().flatten() {
        let Some(photo) = largest_photo(source) else { continue };
        match download(app, photo).await {
            Ok(bytes) => images.push(ImageInput { mime: "image/jpeg".into(), bytes }),
            Err(e) => log::warn!("failed to download photo: {e}"),
        }
    }
    images
}

fn largest_photo(msg: &Message) -> Option<&PhotoSize> {
    msg.photo()?.iter().max_by_key(|p| p.file.size)
}

/// Downloads a photo through the Bot API. The file URL contains the bot token, so it must never be
/// handed to the LLM; the bytes are embedded instead.
async fn download(app: &App, photo: &PhotoSize) -> Result<Vec<u8>, String> {
    let file = app.bot.get_file(photo.file.id.clone()).await.map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(file.size as usize);
    app.bot
        .download_file(&file.path, &mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}
