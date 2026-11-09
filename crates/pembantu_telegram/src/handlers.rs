use std::sync::Arc;

use pembantu_core::{
    agent::{AgentRequest, ChatInfo, ChatKind, ImageInput, Trigger, UserInfo},
    db::{self, NewMessage},
    error::PembantuError,
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

async fn store_incoming(app: &App, msg: &Message) -> Result<(), pembantu_core::error::PembantuError> {
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
    mentioned || msg.text().or(msg.caption()).is_some_and(|t| t.starts_with("AI,"))
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
        Command::Memory(args) => {
            let text = match msg.from.as_ref() {
                Some(user) => memory_command(&app, user.id.0 as i64, args.trim(), lang).await.unwrap_or_else(|e| {
                    log::error!("/memory failed: {e}");
                    i18n::error(lang).to_owned()
                }),
                None => i18n::error(lang).to_owned(),
            };
            reply::send_text(&app, target, truncate_chars(&text, 4000), false).await.map(drop)
        }
    };
    if let Err(e) = result {
        log::error!("failed to handle command in chat {}: {e}", target.chat_id);
    }
}

async fn memory_command(app: &App, user_id: i64, args: &str, lang: Lang) -> Result<String, PembantuError> {
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
    match app.agent.generate_image(prompt).await {
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
    let mut images = Vec::new();
    for source in [Some(msg), msg.reply_to_message()].into_iter().flatten() {
        let Some(photo) = largest_photo(source) else { continue };
        match download(app, photo).await {
            Ok(bytes) => images.push(ImageInput { mime: "image/jpeg".into(), bytes }),
            Err(e) => log::warn!("failed to download photo: {e}"),
        }
    }
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
