//! User-facing strings in Bahasa Indonesia and English.

use siti_core::db::CancelResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Id,
    En,
}

/// Indonesian is the default; English only for English-language clients.
pub fn lang(code: Option<&str>) -> Lang {
    match code {
        Some(code) if code.starts_with("en") => Lang::En,
        _ => Lang::Id,
    }
}

pub fn thinking(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "⏳ Sedang berpikir…",
        Lang::En => "⏳ Thinking…",
    }
}

pub fn error(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Maaf, terjadi kesalahan. Coba lagi nanti.",
        Lang::En => "Sorry, something went wrong. Please try again later.",
    }
}

pub fn tool_status(lang: Lang, name: &str) -> &'static str {
    match (name, lang) {
        ("web_search", Lang::Id) => "🔎 Mencari di web…",
        ("web_search", Lang::En) => "🔎 Searching the web…",
        ("read_webpage", Lang::Id) => "📄 Membaca halaman…",
        ("read_webpage", Lang::En) => "📄 Reading the page…",
        ("read_chat_history", Lang::Id) => "💬 Membaca obrolan…",
        ("read_chat_history", Lang::En) => "💬 Reading the chat…",
        ("generate_image", Lang::Id) => "🖼️ Membuat gambar…",
        ("generate_image", Lang::En) => "🖼️ Creating the image…",
        (_, Lang::Id) => "⚙️ Sedang bekerja…",
        (_, Lang::En) => "⚙️ Working…",
    }
}

macro_rules! help_id {
    () => {
        concat!(
            "Yang bisa saya lakukan:\n",
            "• Ngobrol dan menjawab pertanyaan\n",
            "• Memahami foto yang kamu kirim\n",
            "• Di grup: mention saya, balas pesan saya, atau awali pesan dengan \"Siti,\"\n",
            "• Mengingat isi obrolan grup dan fakta tentang kamu (cukup bilang \"ingat bahwa …\")\n",
            "• Mencari di web dan membaca halaman web\n",
            "• Membuat gambar\n",
            "• Pengingat dan tugas terjadwal (mis. \"ingatkan aku 10 menit lagi untuk minum\")\n\n",
            "Perintah:\n",
            "/ask <pertanyaan> — tanya apa saja\n",
            "/image <deskripsi> — buat gambar\n",
            "/memory — lihat atau hapus hal yang saya ingat tentangmu\n",
            "/summary [jam] — ringkas obrolan (default 24 jam)\n",
            "/briefing HH:MM | off — briefing harian\n",
            "/tasks — daftar pengingat & tugas terjadwal\n",
            "/timezone [zona] — lihat atau ubah zona waktu\n",
            "/help — bantuan"
        )
    };
}

macro_rules! help_en {
    () => {
        concat!(
            "What I can do:\n",
            "• Chat and answer questions\n",
            "• Understand photos you send\n",
            "• In groups: mention me, reply to my message, or start a message with \"Siti,\"\n",
            "• Remember the group conversation and facts about you (just say \"remember that …\")\n",
            "• Search the web and read web pages\n",
            "• Generate images\n",
            "• Reminders and scheduled tasks (e.g. \"remind me in 10 minutes to drink water\")\n\n",
            "Commands:\n",
            "/ask <question> — ask anything\n",
            "/image <description> — generate an image\n",
            "/memory — see or delete what I remember about you\n",
            "/summary [hours] — summarize the chat (default 24 hours)\n",
            "/briefing HH:MM | off — daily briefing\n",
            "/tasks — list reminders & scheduled tasks\n",
            "/timezone [zone] — show or change your timezone\n",
            "/help — help"
        )
    };
}

pub fn welcome(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => concat!(
            "Halo! Saya Siti, asisten AI kamu di Telegram.\n\n",
            help_id!()
        ),
        Lang::En => concat!(
            "Hi! I'm Siti, your AI assistant on Telegram.\n\n",
            help_en!()
        ),
    }
}

pub fn help(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => help_id!(),
        Lang::En => help_en!(),
    }
}

pub fn usage_image(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Cara pakai: /image <deskripsi gambar>",
        Lang::En => "Usage: /image <image description>",
    }
}

pub fn usage_ask(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Cara pakai: /ask <pertanyaan>, atau balas sebuah pesan dengan /ask",
        Lang::En => "Usage: /ask <question>, or reply to a message with /ask",
    }
}

pub fn usage_memory(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Cara pakai: /memory, /memory forget <id>, atau /memory clear",
        Lang::En => "Usage: /memory, /memory forget <id>, or /memory clear",
    }
}

pub fn usage_summary(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Cara pakai: /summary [jam, 1–168]",
        Lang::En => "Usage: /summary [hours, 1–168]",
    }
}

pub fn memory_empty(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Saya belum mengingat apa pun tentangmu.",
        Lang::En => "I don't remember anything about you yet.",
    }
}

pub fn memory_header(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Yang saya ingat tentangmu (hapus dengan /memory forget <id>):",
        Lang::En => "What I remember about you (delete with /memory forget <id>):",
    }
}

pub fn memory_cleared(lang: Lang, count: u64) -> String {
    match lang {
        Lang::Id => format!("{count} ingatan dihapus."),
        Lang::En => format!("Deleted {count} memories."),
    }
}

pub fn memory_forgotten(lang: Lang, id: i64) -> String {
    match lang {
        Lang::Id => format!("Ingatan #{id} dihapus."),
        Lang::En => format!("Forgot memory #{id}."),
    }
}

pub fn memory_not_found(lang: Lang, id: i64) -> String {
    match lang {
        Lang::Id => format!("Ingatan #{id} tidak ditemukan."),
        Lang::En => format!("Memory #{id} not found."),
    }
}

pub fn usage_briefing(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Cara pakai: /briefing HH:MM untuk briefing harian di obrolan ini, atau /briefing off",
        Lang::En => "Usage: /briefing HH:MM for a daily briefing in this chat, or /briefing off",
    }
}

pub fn briefing_name(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Briefing harian",
        Lang::En => "Daily briefing",
    }
}

pub fn briefing_current(lang: Lang, schedule: &str, timezone: &str) -> String {
    match lang {
        Lang::Id => format!("Briefing harian aktif: {schedule} ({timezone}). Matikan dengan /briefing off."),
        Lang::En => format!("Daily briefing is on: {schedule} ({timezone}). Turn it off with /briefing off."),
    }
}

pub fn briefing_set(lang: Lang, time: &str, timezone: &str, next: &str) -> String {
    match lang {
        Lang::Id => format!("Briefing harian dijadwalkan setiap hari pukul {time} ({timezone}). Berikutnya: {next}."),
        Lang::En => format!("Daily briefing scheduled every day at {time} ({timezone}). Next: {next}."),
    }
}

pub fn briefing_off(lang: Lang, was_on: bool) -> &'static str {
    match (lang, was_on) {
        (Lang::Id, true) => "Briefing harian dimatikan.",
        (Lang::Id, false) => "Tidak ada briefing harian yang aktif.",
        (Lang::En, true) => "Daily briefing turned off.",
        (Lang::En, false) => "No daily briefing is active.",
    }
}

pub fn usage_tasks(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Cara pakai: /tasks atau /tasks cancel <id>",
        Lang::En => "Usage: /tasks or /tasks cancel <id>",
    }
}

pub fn tasks_empty(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Tidak ada pengingat atau tugas terjadwal di obrolan ini.",
        Lang::En => "No reminders or scheduled tasks in this chat.",
    }
}

pub fn tasks_header(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Pengingat & tugas terjadwal (batalkan dengan /tasks cancel <id>):",
        Lang::En => "Reminders & scheduled tasks (cancel with /tasks cancel <id>):",
    }
}

pub fn task_cancel_result(lang: Lang, result: CancelResult) -> &'static str {
    match (lang, result) {
        (Lang::Id, CancelResult::Cancelled) => "Tugas dibatalkan.",
        (Lang::Id, CancelResult::NotFound) => "Tugas aktif dengan id itu tidak ditemukan di obrolan ini.",
        (Lang::Id, CancelResult::NotOwner) => "Hanya pembuat tugas yang bisa membatalkannya.",
        (Lang::En, CancelResult::Cancelled) => "Task cancelled.",
        (Lang::En, CancelResult::NotFound) => "No active task with that id in this chat.",
        (Lang::En, CancelResult::NotOwner) => "Only the task's creator can cancel it.",
    }
}

pub fn timezone_current(lang: Lang, timezone: &str) -> String {
    match lang {
        Lang::Id => format!("Zona waktumu: {timezone}. Ubah dengan /timezone <nama IANA>, mis. /timezone Asia/Makassar."),
        Lang::En => format!("Your timezone: {timezone}. Change it with /timezone <IANA name>, e.g. /timezone Europe/Berlin."),
    }
}

pub fn timezone_set(lang: Lang, timezone: &str) -> String {
    match lang {
        Lang::Id => format!("Zona waktu diatur ke {timezone}."),
        Lang::En => format!("Timezone set to {timezone}."),
    }
}

pub fn timezone_invalid(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => "Zona waktu tidak dikenal. Gunakan nama IANA, contoh: Asia/Jakarta",
        Lang::En => "Unknown timezone. Use an IANA name, for example: Asia/Jakarta",
    }
}
