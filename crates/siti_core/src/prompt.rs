use std::fmt::Write;

use chrono::DateTime;
use chrono_tz::Tz;

use crate::agent::{ChatInfo, ChatKind, UserInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyLanguage {
    Indonesian,
    English,
}

impl ReplyLanguage {
    /// From a Telegram `language_code`, used when the message itself is inconclusive.
    /// Indonesian is the default; English only for English-language clients.
    pub fn from_code(code: Option<&str>) -> Self {
        match code {
            Some(code) if code.starts_with("en") => ReplyLanguage::English,
            _ => ReplyLanguage::Indonesian,
        }
    }

    fn name(self) -> &'static str {
        match self {
            ReplyLanguage::Indonesian => "Bahasa Indonesia",
            ReplyLanguage::English => "English",
        }
    }
}

const INDONESIAN_WORDS: &[&str] = &[
    "aku", "saya", "kamu", "anda", "kau", "yang", "dan", "untuk", "dengan", "ini", "itu", "apa", "apakah",
    "bagaimana", "gimana", "kenapa", "mengapa", "tolong", "bisa", "lagi", "menit", "detik", "jam", "besok",
    "hari", "ingatkan", "ingat", "cari", "carikan", "buat", "buatkan", "sudah", "belum", "tidak", "nggak",
    "gak", "ga", "dong", "deh", "sih", "ke", "dari", "di", "pada", "juga", "akan", "mau", "ada", "berapa",
    "siapa", "kapan", "mana", "sekarang", "terima", "kasih", "halo", "selamat", "pagi", "siang", "malam",
    "minum", "makan", "jelaskan", "ringkas", "rangkum", "berita", "tentang",
];

const ENGLISH_WORDS: &[&str] = &[
    "the", "a", "an", "you", "i", "me", "my", "to", "in", "is", "are", "was", "what", "how", "why", "who",
    "when", "where", "please", "remind", "remember", "can", "could", "would", "and", "for", "of", "on", "at",
    "with", "this", "that", "it", "do", "does", "minutes", "seconds", "hours", "tomorrow", "today", "hi",
    "hello", "thanks", "search", "find", "summarize", "about", "news", "latest", "give", "tell", "write",
];

/// Guesses whether `text` is Indonesian or English by counting common words; `None` when unclear.
/// The lite models drift into Indonesian after tool calls (Indonesian name, Jakarta clock), so the
/// reply language is decided here and stated explicitly instead of left to the model.
pub fn detect_language(text: &str) -> Option<ReplyLanguage> {
    let (mut id, mut en) = (0, 0);
    for word in text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let word = word.to_lowercase();
        id += usize::from(INDONESIAN_WORDS.contains(&word.as_str()));
        en += usize::from(ENGLISH_WORDS.contains(&word.as_str()));
    }
    match id.cmp(&en) {
        std::cmp::Ordering::Greater => Some(ReplyLanguage::Indonesian),
        std::cmp::Ordering::Less => Some(ReplyLanguage::English),
        std::cmp::Ordering::Equal => None,
    }
}

pub struct PromptContext<'a> {
    pub now_local: DateTime<Tz>,
    pub timezone: Tz,
    pub chat: &'a ChatInfo,
    pub user: &'a UserInfo,
    pub memories: &'a [(i64, String)],
    pub tool_names: &'a [&'a str],
    pub language: ReplyLanguage,
    /// Personality set for this chat with /persona.
    pub persona: Option<&'a str>,
}

pub fn system_prompt(p: &PromptContext) -> String {
    let mut s = String::from(
        "You are Siti, a personal AI assistant that lives in Telegram and gets things done. \
         Prefer acting with your tools over telling the user how to do something. Be concise.\n",
    );
    if let Some(persona) = p.persona {
        let _ = writeln!(
            s,
            "Persona: an admin of this chat set your personality. Stay in this character in tone and style for \
             every reply, while still being genuinely helpful and following the rules below:\n{persona}"
        );
    }
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
        s.push_str(
            "Use read_chat_history before summarizing. \
             When the user states a lasting personal fact or preference, call remember.\n",
        );
        if p.tool_names.contains(&"create_reminder") {
            let _ = writeln!(
                s,
                "Times the user gives are in {}; use create_reminder for fixed messages and schedule_agent_task \
                 when work must be done at that time (e.g. a daily briefing).",
                p.timezone
            );
        }
    }
    // Last, so it outweighs the Indonesian name, the timezone and earlier replies.
    let _ = write!(
        s,
        "Language: the user's latest message is in {0}. Write your whole reply in {0}, also after using tools.",
        p.language.name()
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_reply_language() {
        let cases = [
            ("remind me in 20 seconds to drink water", Some(ReplyLanguage::English)),
            ("remember that I like tea", Some(ReplyLanguage::English)),
            ("what's the latest news about Rust?", Some(ReplyLanguage::English)),
            ("ingatkan aku 30 detik lagi untuk minum", Some(ReplyLanguage::Indonesian)),
            ("Halo, apa kabar? Jawab singkat saja.", Some(ReplyLanguage::Indonesian)),
            ("tolong carikan berita tentang Rust", Some(ReplyLanguage::Indonesian)),
            ("2+2?", None),
            ("ok", None),
        ];
        for (text, expected) in cases {
            assert_eq!(detect_language(text), expected, "{text}");
        }
    }
}
