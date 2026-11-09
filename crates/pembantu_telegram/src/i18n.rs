//! User-facing strings in Bahasa Indonesia and English.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Id,
    En,
}

pub fn lang(code: Option<&str>) -> Lang {
    match code {
        Some(code) if code.starts_with("id") => Lang::Id,
        _ => Lang::En,
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
            "• Di grup: mention saya, balas pesan saya, atau awali pesan dengan \"AI,\"\n",
            "• Mengingat isi obrolan grup dan fakta tentang kamu (cukup bilang \"ingat bahwa …\")\n",
            "• Mencari di web dan membaca halaman web\n",
            "• Membuat gambar\n\n",
            "Perintah:\n",
            "/ask <pertanyaan> — tanya apa saja\n",
            "/image <deskripsi> — buat gambar\n",
            "/memory — lihat atau hapus hal yang saya ingat tentangmu\n",
            "/summary [jam] — ringkas obrolan (default 24 jam)\n",
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
            "• In groups: mention me, reply to my message, or start a message with \"AI,\"\n",
            "• Remember the group conversation and facts about you (just say \"remember that …\")\n",
            "• Search the web and read web pages\n",
            "• Generate images\n\n",
            "Commands:\n",
            "/ask <question> — ask anything\n",
            "/image <description> — generate an image\n",
            "/memory — see or delete what I remember about you\n",
            "/summary [hours] — summarize the chat (default 24 hours)\n",
            "/help — help"
        )
    };
}

pub fn welcome(lang: Lang) -> &'static str {
    match lang {
        Lang::Id => concat!(
            "Halo! Saya Pembantu, asisten AI kamu di Telegram.\n\n",
            help_id!()
        ),
        Lang::En => concat!(
            "Hi! I'm Pembantu, your AI assistant on Telegram.\n\n",
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
