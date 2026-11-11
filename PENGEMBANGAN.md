# Pengembangan Siti untuk Aplikasi Chatbot & Agent

Dokumen ini merangkum kondisi Siti saat ini, arsitektur, dan **implementasi pengembangan** yang telah dikerjakan agar Siti menjadi fondasi **aplikasi chatbot dan agent** lintas kanal (Telegram, WhatsApp, Web/API, dsb.).

## 1. Gambaran Siti Saat Ini

Siti adalah asisten AI berbasis Rust dengan arsitektur modular:

- **`siti_core`** — inti agent dan domain logic
  - **Agent loop** (`agent.rs`): menerima `AgentRequest`, membangun system prompt, memanggil LLM via chat-completions (OpenAI-compatible), menjalankan tool-calling berulang hingga jawaban final (batas `MAX_STEPS`), dan menyimpan konteks.
  - **LLM client** (`llm.rs`): provider OpenRouter & Gemini (endpoint OpenAI-compatible), mendukung `tool_calls` dan meneruskan field spesifik provider (mis. `thought_signature`, `reasoning_details`).
  - **Prompt & persona** (`prompt.rs`): deteksi bahasa (ID/EN), persona per-chat, injeksi waktu/timezone, memori user, dan daftar tool.
  - **Persistence Postgres** (`db.rs` + migrasi): user/chat/message, memori per-user, timezone, persona chat, scheduled tasks (once/daily) dengan claim idempotent.
  - **Tools bawaan** (`tools/`): `remember`, `forget`, `read_chat_history`, `set_timezone`, `create_reminder`, `schedule_agent_task`, `list_scheduled_tasks`, `cancel_scheduled_task`, `web_search`, `read_webpage` (Tavily), `generate_image` (Gemini).
  - **Scheduler logic** (`schedule.rs`): representasi dan kalkulasi jadwal/next-run.
- **`siti_telegram`** — channel Telegram (Teloxide): adapter ingest/respons/command/render/scheduler.
- **`siti_whatsapp`** — channel WhatsApp (Meta Cloud API webhook).
- **`siti_api`** — HTTP API untuk integrasi aplikasi chatbot/agent.
- **Infra**: Dockerfile, docker-compose (db+bot), workflow deploy/tag image GHCR.

## 2. Arsitektur Target (diimplementasikan bertahap)

```
                    +----------------------+
                    |   Channel / Clients  |
  Telegram Bot  Web Chat  WhatsApp  Mobile/3rd-party App
        \            |         |            /
         \           |         |           /
          +----------v---------v----------+
          |        Siti Interface         |
          |  (HTTP API / Webhook / SSE)   |
          +---------------+---------------+
                          |
                  +-------v--------+
                  |    siti_core   |
                  | Agent + Tools  |
                  | Memory/Tasks   |
                  | Knowledge/RAG  |
                  +-------+--------+
                          |
                    Postgres + LLM
```

Prinsip: **core sebagai engine**, channel adalah adapter tipis (Telegram, WhatsApp, API).

## 3. Perubahan yang Telah Diimplementasikan

### 3.1 Core (`siti_core`)

- **Generalisasi multi-channel & session** (`conversation.rs`, migrasi `0004_multichannel_and_agent.sql`):
  - Tabel `conversations` (channel + external_id, persona/metadata), `conversation_participants`, `conversation_messages` untuk percakapan generik lintas kanal.
  - Tabel `api_keys` (fondasi auth), `knowledge_documents` (basis pengetahuan/RAG), `agent_runs` (observability/audit run).
  - Helper: ensure/get/list conversation, upsert participant, simpan & ambil pesan, logging agent run.
- **Tool registry/plugin & extensibility** (`tools/mod.rs`):
  - `Tools` kini berfungsi sebagai registry: built-in tools + dukungan **dynamic tool registration** (`DynamicTool`, `ToolRegistry`, `register_dynamic`) agar kapabilitas agent dapat diperluas tanpa mengubah dispatch built-in.
  - API publik utama (`Tools::new`, `names`, `definitions`, `execute`, `image`) dipertahankan untuk kompatibilitas channel yang ada.
- **Knowledge / RAG ringan** (`knowledge.rs`, `tools/knowledge_search.rs`):
  - Modul ingest/search/list/delete dokumen pengetahuan (Postgres, pencarian berbasis `ILIKE`/teks) + tool bawaan **`knowledge_search`** agar agent dapat menjawab berbasis knowledge base.
- **Ekspor modul baru** (`lib.rs`): `conversation`, `knowledge`.

> Catatan kompatibilitas: skema Telegram lama (`users/chats/messages`, `telegram_message_id`) tetap dipertahankan. Adapter baru memetakan identitas kanal ke tabel generik sekaligus menjaga kompatibilitas memori/riwayat core.

### 3.2 HTTP API (`siti_api`)

Layanan HTTP (Axum) sebagai antarmuka chatbot & agent:

- `GET /health` — health check.
- **Sessions/conversations**:
  - `POST /v1/sessions` — buat/ensure session percakapan generik (channel/external id/participant).
  - `GET /v1/sessions` — daftar sesi (filter `channel`, `limit`).
  - `GET /v1/sessions/{id}` — detail sesi + riwayat pesan.
- **Knowledge base**:
  - `POST /v1/knowledge` — tambah dokumen.
  - `GET /v1/knowledge?q=...` — cari dokumen.
  - `DELETE /v1/knowledge/{id}` — hapus dokumen.
- **Chat**:
  - `POST /v1/chat` — jalankan agent (by `conversation_id`/session atau pasangan `user_id`+`chat_id`), mengembalikan `text`, `attachments` (metadata), `tools_used`, `conversation_id`.
  - `POST /v1/chat/stream` — **SSE streaming** event agent: `start`, `tool_started`, `final`, `error`, `done` (event tool real-time; jawaban final dikirim saat selesai — fondasi untuk streaming token penuh).
- **Keamanan & kontrol**: auth sederhana via `X-API-Key`/`Authorization: Bearer` (global key, `REQUIRE_AUTH`) dan **rate limiting** in-memory per key/IP (`RATE_LIMIT_PER_MINUTE`).
- **Observability**: pencatatan `agent_runs` (durasi, tools, sukses/error).
- Konfigurasi via env (`crates/siti_api/.env.example`), CORS untuk integrasi web.

### 3.3 WhatsApp Channel (`siti_whatsapp`)

Implementasi awal adapter **WhatsApp Cloud API (Meta)**:

- Webhook Axum: `GET /webhook` (verifikasi `hub.challenge` + verify token) dan `POST /webhook` (terima pesan masuk).
- Parsing pesan teks masuk, pemetaan ke `conversations` generik (channel=`whatsapp`, external id = nomor pengirim) + penyimpanan riwayat.
- Eksekusi `siti_core::Agent` dan pengiriman balasan via Graph API (`/{phone_number_id}/messages`).
- Fallback aman: jika kredensial kirim belum dikonfigurasi, pesan tetap diproses & dicatat (balasan di-log) agar pengembangan/integrasi mudah.
- Konfigurasi via env (`crates/siti_whatsapp/.env.example`).

### 3.4 Kompatibilitas & Operasional

- **Workspace** (`Cargo.toml`): menambahkan `siti_api`.
- **Telegram** (`siti_telegram`) tetap berfungsi sebagai channel adapter di atas core yang sama (perubahan core menjaga API/tool behavior).
- Migrasi DB bersifat **additive** (`0004_multichannel_and_agent.sql`) agar aman terhadap deployment bertahap.

## 4. Cara Menjalankan

Prasyarat: Rust toolchain, Postgres, kredensial LLM.

```bash
# dari root repo
docker compose up -d db
```

### API Chat/Agent
```bash
# env: crates/siti_api/.env.example
cargo run -p siti_api
```

Contoh (chat langsung):
```bash
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"user_id":1,"chat_id":1001,"user_name":"Budi","message":"Halo, ingatkan aku minum air tiap hari jam 9 pagi"}'
```

Contoh (session + streaming SSE):
```bash
# buat session
curl -X POST http://localhost:3000/v1/sessions -H "Content-Type: application/json" -d '{"user_name":"Budi"}'
# stream chat (SSE)
curl -N -X POST http://localhost:3000/v1/chat/stream -H "Content-Type: application/json" \
  -d '{"conversation_id":1,"user_name":"Budi","message":"Ringkas kabar terbaru tentang Rust"}'
```

Knowledge:
```bash
curl -X POST http://localhost:3000/v1/knowledge -H "Content-Type: application/json" \
  -d '{"title":"FAQ Produk","content":"Siti adalah asisten AI internal...","source":"docs"}'
```

### WhatsApp (Meta Cloud API)
```bash
# env: crates/siti_whatsapp/.env.example (VERIFY_TOKEN, ACCESS_TOKEN, PHONE_NUMBER_ID)
cargo run -p siti_whatsapp
# expose /webhook ke publik (mis. reverse proxy/tunnel) dan daftarkan di Meta App
```

### Telegram (tetap)
```bash
cargo run -p siti_telegram
```

## 5. Catatan Lanjutan (penyempurnaan berikutnya)

Meski fitur utama sudah diimplementasikan, area berikut dapat dimatangkan lebih jauh:

1. **Identitas & idempotency penuh lintas-channel**: namespace terpadu di seluruh core (mengurangi ketergantungan pada pemetaan `telegram_message_id`/id sintetis) dan penyatuan riwayat ke tabel generik end-to-end.
2. **Streaming token penuh**: dukungan streaming dari provider LLM ke SSE/WebSocket (saat ini event + final).
3. **Plugin tool yang lebih kaya**: tool dari definisi/schema eksternal (HTTP/webhook tool) dan kebijakan allow/deny per channel/user/tenant.
4. **Auth & multi-tenant produksi**: API key per-tenant di DB (`api_keys`), JWT/OAuth, scoping data ketat, rate limiting terdistribusi & quota.
5. **Workflows/goal persisten & guardrails**: state workflow jangka panjang, batas step/biaya per request, kebijakan safety/tool.
6. **RAG lanjutan**: chunking, embedding/vector search, dan sumber pengetahuan terkelola.
7. **Observability & kualitas**: tracing terstruktur (OpenTelemetry), metrik, dashboard, dan evaluasi/regression test agent & tools.
8. **WhatsApp lanjutan**: dukungan media (gambar/audio/dokumen), template/interaktif, verifikasi signature webhook, dan retry/idempotency delivery.