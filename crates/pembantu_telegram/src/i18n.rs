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
            "• Di grup: mention saya, balas pesan saya, atau awali pesan dengan \"AI,\"\n\n",
            "Perintah:\n",
            "/ask <pertanyaan> — tanya apa saja\n",
            "/image <deskripsi> — buat gambar\n",
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
            "• In groups: mention me, reply to my message, or start a message with \"AI,\"\n\n",
            "Commands:\n",
            "/ask <question> — ask anything\n",
            "/image <description> — generate an image\n",
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
