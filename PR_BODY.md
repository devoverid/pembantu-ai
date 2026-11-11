## Summary
Mengembangkan Siti dari bot Telegram menjadi fondasi aplikasi **chatbot & agent** lintas kanal, dengan core yang lebih general, HTTP API, dan adapter WhatsApp.

## Changes
### Core (`siti_core`)
- Migrasi additive `0004_multichannel_and_agent.sql`: tabel `conversations`, `conversation_participants`, `conversation_messages`, `api_keys`, `knowledge_documents`, `agent_runs`.
- Modul `conversation` & `knowledge`: session/percakapan generik, penyimpanan pesan, logging run, dan operasi knowledge base.
- Tool registry/plugin: `Tools` mendukung dynamic tool registration (`DynamicTool`/`ToolRegistry`) dengan API publik utama tetap kompatibel.
- Tool baru `knowledge_search` (RAG ringan) terintegrasi ke agent.

### API (`siti_api`)
- Layanan HTTP (Axum): `GET /health`.
- Sessions: `POST/GET /v1/sessions`, `GET /v1/sessions/{id}`.
- Knowledge: `POST/GET/DELETE /v1/knowledge`.
- Chat: `POST /v1/chat` (sinkron) dan `POST /v1/chat/stream` (SSE: `start`, `tool_started`, `final`, `error`, `done`).
- Auth sederhana (API key/Bearer), rate limiting in-memory, CORS, dan pencatatan `agent_runs`.

### WhatsApp (`siti_whatsapp`)
- Implementasi awal adapter Meta WhatsApp Cloud API: webhook verifikasi/penerimaan pesan, pemrosesan via core agent, persistensi percakapan, pengiriman balasan Graph API (fallback aman bila kredensial kirim belum lengkap).

### Docs & workspace
- `PENGEMBANGAN.md`, README crate (`siti_api`, `siti_whatsapp`), `.env.example`, dan pendaftaran `siti_api` di workspace.

## How to run
```bash
docker compose up -d db
cargo run -p siti_api        # lihat crates/siti_api/.env.example
cargo run -p siti_whatsapp   # lihat crates/siti_whatsapp/.env.example
cargo run -p siti_telegram   # tetap
```

## Notes
- Migrasi DB additive; alur Telegram dipertahankan.
- Identitas/idempotency lintas-channel dan streaming token penuh dari provider LLM masih dapat dimatangkan lebih lanjut (lihat `PENGEMBANGAN.md`).
- Disarankan verifikasi: `cargo check -p siti_core -p siti_api -p siti_whatsapp -p siti_telegram`.
