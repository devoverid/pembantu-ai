use std::fmt::Write;

use chrono::DateTime;
use chrono_tz::Tz;

use crate::agent::{ChatInfo, ChatKind, UserInfo};

pub struct PromptContext<'a> {
    pub now_local: DateTime<Tz>,
    pub timezone: Tz,
    pub chat: &'a ChatInfo,
    pub user: &'a UserInfo,
    pub memories: &'a [(i64, String)],
    pub tool_names: &'a [&'a str],
}

pub fn system_prompt(p: &PromptContext) -> String {
    let mut s = String::from(
        "You are Pembantu, a personal AI assistant that lives in Telegram and gets things done. \
         Reply in the language the user writes in (Bahasa Indonesia or English).\n\
         Prefer acting with your tools over telling the user how to do something. Be concise.\n",
    );
    let _ = writeln!(s, "Current time: {} ({}).", p.now_local.format("%A, %Y-%m-%d %H:%M"), p.timezone);
    match p.chat.kind {
        ChatKind::Private => {
            let _ = writeln!(s, "You are in a private chat with {}.", p.user.full_name);
        }
        ChatKind::Group => {
            let _ = writeln!(
                s,
                "You are in the group \"{}\". Messages are prefixed with the sender's name; the current request is from {}.",
                p.chat.title.as_deref().unwrap_or("unnamed group"),
                p.user.full_name
            );
        }
    }
    s.push_str(
        "Format with Markdown only: **bold**, _italic_, `code`, fenced code blocks, [links](url), bullet lists. \
         No tables or headings.\n",
    );
    if !p.memories.is_empty() {
        let _ = writeln!(s, "Known facts about {}:", p.user.full_name);
        for (id, fact) in p.memories {
            let _ = writeln!(s, "- [#{id}] {fact}");
        }
    }
    if !p.tool_names.is_empty() {
        let _ = writeln!(s, "Available tools: {}.", p.tool_names.join(", "));
    }
    s.trim_end().to_owned()
}
