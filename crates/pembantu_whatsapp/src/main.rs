use dotenv::dotenv;
use pembantu_core::provider::{ImageProvider, TextProvider};
use secrecy::ExposeSecret;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use wacore::types::events::Event;
use whatsapp_rust::bot::Bot;
use whatsapp_rust::pair_code::PairCodeOptions;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

pub mod command;
pub mod conversation;
pub mod types;

use crate::command::Command;
use crate::conversation::Conversation;
use crate::types::Config;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    println!("🦀 Memulai WhatsApp Bot (Modular Refactor)...\n");

    let env_config = envy::from_env::<Config>().expect("Failed to parse env");

    // Init AI Providers
    let text_provider = match env_config.provider_text_generation {
        None => None,
        Some(s) => Some(match s.as_str() {
            "openrouter" => TextProvider::OpenRouter(
                env_config.openrouter_api_key.expose_secret().to_string(),
                env_config
                    .model_text_generation
                    .expect("MODEL_TEXT_GENERATION is empty"),
            ),
            "gemini" => TextProvider::Gemini(
                env_config.gemini_api_key.expose_secret().to_string(),
                env_config
                    .model_text_generation
                    .expect("MODEL_TEXT_GENERATION is empty"),
            ),
            _ => {
                panic!("Text generation model not supported. Available options: openrouter, gemini")
            }
        }),
    };
    let image_provider = match env_config.provider_image_generation {
        None => None,
        Some(s) => Some(match s.as_str() {
            "gemini" => ImageProvider::Gemini(
                env_config.gemini_api_key.expose_secret().to_string(),
                env_config
                    .model_image_generation
                    .expect("MODEL_IMAGE_GENERATION is empty"),
            ),
            _ => panic!("Image generation model not supported. Available options: gemini"),
        }),
    };

    let ai_bot = pembantu_core::bot::Bot::new(text_provider, image_provider)
        .expect("Failed to create AI Bot");
    let conversation = Arc::new(Conversation::new(ai_bot));

    let data_dir = Path::new("crates/pembantu_whatsapp/data");
    std::fs::create_dir_all(&data_dir).expect("Gagal membuat folder data");

    let db_path = data_dir.join("whatsapp.db");
    let db_path_str = db_path.to_str().expect("Path tidak valid");

    let has_session = db_path.exists()
        && std::fs::metadata(&db_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false);

    println!("📂 Database disimpan di: {}", db_path_str);

    let backend = Arc::new(
        SqliteStore::new(db_path_str)
            .await
            .expect("Gagal membuat database SQLite"),
    );

    let mut builder = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new());

    if !has_session {
        println!("📝 Belum ada session yang tersimpan.");
        print!("Masukkan nomor telepon (contoh: 628123456789): ");
        io::stdout().flush().expect("Gagal flush stdout");

        let mut phone_number = String::new();
        io::stdin()
            .read_line(&mut phone_number)
            .expect("Gagal membaca input");
        let phone_number = phone_number.trim().to_string();

        if phone_number.is_empty() {
            println!("❌ Nomor telepon tidak boleh kosong!");
            return;
        }

        println!("\n📱 Menggunakan nomor: {}", phone_number);
        println!("⏳ Memulai proses pairing...\n");

        builder = builder.with_pair_code(PairCodeOptions {
            phone_number,
            custom_code: None,
            ..Default::default()
        });
    } else {
        println!("✅ Session ditemukan! Menghubungkan ke WhatsApp...\n");
    }

    let conversation_handler = conversation.clone();
    let mut bot = builder
        .on_event(move |event, client| {
            let conversation_handler = conversation_handler.clone();
            async move {
                match event {
                    Event::PairingCode { code, timeout } => {
                        println!("========================================");
                        println!("📱 KODE PAIRING (berlaku {} detik):", timeout.as_secs());
                        println!("\nMasukkan kode ini di HP Anda:");
                        println!("WhatsApp > Perangkat Tertaut > Tautkan Perangkat");
                        println!("> Tautkan dengan nomor telepon");
                        let code_parts: Vec<&str> = code
                            .as_str()
                            .as_bytes()
                            .chunks(4)
                            .map(|c| std::str::from_utf8(c).unwrap())
                            .collect();
                        println!("    >>> {} <<<", code_parts.join("-"));
                        println!("========================================");
                    }
                    Event::PairingQrCode { code, .. } => {
                        println!("\n📱 QR CODE (alternatif):");
                        println!("{}", code);
                    }
                    Event::Message(msg, info) => {
                        if let Some(text) = msg.conversation.as_ref() {
                            println!("\n💬 Pesan dari {}: {}", info.source.sender, text);

                            // Parse command
                            if let Some(cmd) = Command::from_message(text) {
                                log::info!("Command detected: {:?}", cmd);
                                conversation_handler
                                    .handle_command(&client, &info, cmd)
                                    .await;
                            } else {
                                // Optional: Handle non-command messages (e.g., auto-reply if mentioned?)
                            }
                        }
                    }
                    Event::Connected(_) => {
                        println!("\n✅ Terhubung ke WhatsApp!");
                    }
                    Event::Disconnected(_) => {
                        println!("\n❌ Terputus dari WhatsApp");
                    }
                    _ => {}
                }
            }
        })
        .build()
        .await
        .expect("Gagal membuat bot");

    println!("🚀 Bot siap! Tekan Ctrl+C untuk keluar.\n");

    bot.run()
        .await
        .expect("Gagal menjalankan bot")
        .await
        .expect("Bot berhenti dengan error");
}
