use crate::command::Command;
use pembantu_core::bot::Bot;
use wacore::types::message::MessageInfo;
use wacore_binary::jid::Jid;
use waproto::whatsapp as wa;
use whatsapp_rust::client::Client;

pub struct Conversation {
    bot: Bot,
}

impl Conversation {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }

    pub async fn handle_command(&self, client: &Client, info: &MessageInfo, cmd: Command) {
        let chat_id = info.source.chat.clone();

        match cmd {
            Command::Help => {
                let text = "🤖 Bantuan Pembantu WhatsApp:\n\n/ask <pertanyaan> - Bertanya ke AI\n/image <prompt> - Membuat gambar AI\n/ping - Cek status bot\n/speed - Cek kecepatan bot";
                self.send_text(client, &chat_id, text).await;
            }
            Command::Ping => {
                self.send_text(client, &chat_id, "Pong! 🦀").await;
            }
            Command::Speed => {
                self.send_text(client, &chat_id, "⚡ Speed test initiated...")
                    .await;
            }
            Command::Ask(query) => {
                if query.is_empty() {
                    self.send_text(
                        client,
                        &chat_id,
                        "❌ Mohon sertakan pertanyaan. Contoh: /ask halo",
                    )
                    .await;
                    return;
                }
                self.generate_and_send_text(client, &chat_id, query).await;
            }
            Command::Image(prompt) => {
                if prompt.is_empty() {
                    self.send_text(
                        client,
                        &chat_id,
                        "❌ Mohon sertakan prompt gambar. Contoh: /image kucing lucu",
                    )
                    .await;
                    return;
                }
                self.generate_and_send_image(client, &chat_id, prompt).await;
            }
            Command::Unknown => {
                // Ignore unknown commands
            }
        }
    }

    async fn send_text(&self, client: &Client, chat_id: &Jid, text: &str) {
        let msg = wa::Message {
            conversation: Some(text.to_string()),
            ..Default::default()
        };
        if let Err(e) = client.send_message(chat_id.clone(), msg).await {
            log::error!("Failed to send message: {:?}", e);
        }
    }

    pub async fn generate_and_send_text(&self, client: &Client, chat_id: &Jid, text: String) {
        log::info!("Generating text for: {}", text);

        match self.bot.generate_text(text, None).await {
            Ok(response) => {
                self.send_text(client, chat_id, &response).await;
            }
            Err(e) => {
                log::error!("Error generating text: {:?}", e);
                self.send_text(
                    client,
                    chat_id,
                    "❌ Maaf, sedang ada gangguan pada layanan AI.",
                )
                .await;
            }
        }
    }

    pub async fn generate_and_send_image(&self, client: &Client, chat_id: &Jid, prompt: String) {
        log::info!("Generating image for: {}", prompt);

        match self.bot.generate_image(prompt).await {
            Ok(_image_data) => {
                self.send_text(client, chat_id, "⚠️ Fitur generate image belum sepenuhnya diimplementasikan di adapter WhatsApp ini.").await;
            }
            Err(e) => {
                log::error!("Error generating image: {:?}", e);
                self.send_text(
                    client,
                    chat_id,
                    "❌ Maaf, sedang ada gangguan pada layanan AI.",
                )
                .await;
            }
        }
    }
}
