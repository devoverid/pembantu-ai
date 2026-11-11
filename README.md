<h1 align="center">Siti</h1>
<p align="center">AI chatbot & agent untuk Telegram, WhatsApp, dan integrasi aplikasi (HTTP API)</p>

<p align="center">
  Coba bot Telegram: <a href="https://t.me/PembantuAI_Bot">@PembantuAI_Bot</a>
</p>

<p align="center">
  <img align="center" src="https://github.com/devoverid/pembantu-ai/assets/45036724/f6a0e236-f816-4402-91de-7ab467d43573" width="420" alt="Siti preview">
</p>

Siti adalah asisten AI berbasis Rust dengan **core agent** (tool-calling, memori, penjadwalan, knowledge/RAG) yang dapat dipakai lintas kanal: Telegram, WhatsApp (Meta Cloud API), dan aplikasi lain melalui HTTP API.

> Detail arsitektur dan roadmap: lihat [`PENGEMBANGAN.md`](PENGEMBANGAN.md).

## Fitur

- **Chatbot & agent**: loop agent dengan tool-calling, konteks percakapan, deteksi bahasa, persona per-chat.
- **Memori & personalisasi**: fakta/preferensi pengguna (`remember/forget`), timezone, persona.
- **Penjadwalan**: reminder dan agent task terjadwal (sekali/harian), briefing.
- **Tools bawaan**: riwayat chat, web search & baca halaman (Tavily), generate image (Gemini), pencarian knowledge base.
- **Knowledge base (RAG ringan)**: ingest dan pencarian dokumen untuk menjawab berbasis pengetahuan.
- **Multi-channel**:
  - **Telegram** (`siti_telegram`)
  - **WhatsApp Cloud API** (`siti_whatsapp`, webhook)
  - **HTTP API** (`siti_api`) untuk integrasi web/mobile/backend (session, chat sinkron, SSE streaming, knowledge endpoint)
- **Operasional dasar**: migrasi Postgres, auth API key & rate limit (API), logging run agent.

## Struktur Crates

| Crate | Deskripsi |
| --- | --- |
| `siti_core` | Engine agent, LLM client (OpenRouter/Gemini), tools/registry, DB, memori, scheduler, knowledge, percakapan generik |
| `siti_telegram` | Adapter/channel Telegram (Teloxide, command, render, scheduler worker) |
| `siti_api` | Layanan HTTP untuk chatbot & agent (sessions, chat, SSE, knowledge, auth/rate limit) |
| `siti_whatsapp` | Adapter/channel WhatsApp Meta Cloud API (webhook + kirim pesan) |

## Prasyarat

- Rust toolchain
- PostgreSQL
- Kredensial provider LLM (OpenRouter atau Gemini), dan opsional Tavily / image generation sesuai kebutuhan

## Menjalankan Secara Lokal

### 1. Clone & siapkan environment

```bash
git clone https://github.com/devoverid/siti.git
cd siti
cp .env.example .env
```

Isi `.env` (contoh minimal untuk Telegram/LLM):

```env
# Telegram
TELOXIDE_TOKEN=

# Core / DB
RUST_LOG=info
DEFAULT_TIMEZONE=Asia/Jakarta
DATABASE_URL=postgres://siti:siti@localhost:5432/siti

# Text Generation
PROVIDER_TEXT_GENERATION=openrouter
MODEL_TEXT_GENERATION=google/gemini-3.5-flash-lite

# API Keys
OPENROUTER_API_KEY=
PROVIDER_IMAGE_GENERATION=openrouter
MODEL_IMAGE_GENERATION=bytedance-seed/seedream-5-0-flash
GEMINI_API_KEY=
TAVILY_API_KEY=
```

> Untuk group chat Telegram, nonaktifkan privacy mode di @BotFather (`/setprivacy` → Disable) agar bot bisa membaca pesan grup.

### 2. Jalankan Postgres

```bash
docker compose up -d db
```

Jika port 5432 bentrok, sesuaikan `POSTGRES_PORT` dan `DATABASE_URL`.

### 3. Jalankan service yang dibutuhkan

**Telegram bot**
```bash
cargo run -p siti_telegram
```

**HTTP API (chatbot & agent)**
```bash
# env khusus: crates/siti_api/.env.example (API_HOST/PORT, API_KEY, RATE_LIMIT_PER_MINUTE, dll.)
cargo run -p siti_api
```

**WhatsApp (Meta Cloud API webhook)**
```bash
# env khusus: crates/siti_whatsapp/.env.example (WHATSAPP_VERIFY_TOKEN, WHATSAPP_ACCESS_TOKEN, WHATSAPP_PHONE_NUMBER_ID, dll.)
cargo run -p siti_whatsapp
```

## Contoh Penggunaan API

```bash
# Chat sinkron
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"user_id":1,"chat_id":1001,"user_name":"Budi","message":"Halo, bantu saya"}'

# Buat session lalu chat via SSE streaming
curl -X POST http://localhost:3000/v1/sessions -H "Content-Type: application/json" -d '{"user_name":"Budi"}'
curl -N -X POST http://localhost:3000/v1/chat/stream -H "Content-Type: application/json" \
  -d '{"conversation_id":1,"user_name":"Budi","message":"Ringkas kabar terbaru tentang Rust"}'

# Knowledge base
curl -X POST http://localhost:3000/v1/knowledge -H "Content-Type: application/json" \
  -d '{"title":"FAQ Produk","content":"...","source":"docs"}'
```

Lihat [`crates/siti_api/README.md`](crates/siti_api/README.md) dan [`crates/siti_whatsapp/README.md`](crates/siti_whatsapp/README.md) untuk detail endpoint/konfigurasi.

## Menjalankan dengan Docker Image (Telegram)

`docker-compose.prod.yml` menjalankan image prebuilt `ghcr.io/devoverid/siti:1.1.1` bersama Postgres (tanpa build lokal).

1. Buat `.env` dari `.env.example` dan isi nilainya (`DATABASE_URL` di-set oleh compose).
   ```bash
   cp .env.example .env
   ```
2. Jalankan stack:
   ```bash
   docker compose -f docker-compose.prod.yml up -d
   ```
3. Lihat log bot:
   ```bash
   docker compose -f docker-compose.prod.yml logs -f bot
   ```
4. Hentikan:
   ```bash
   docker compose -f docker-compose.prod.yml down
   ```

Upgrade: ubah tag `image` di `docker-compose.prod.yml`, lalu jalankan `docker compose -f docker-compose.prod.yml pull && docker compose -f docker-compose.prod.yml up -d`.

## Rilis Docker Image

Push tag `vMAJOR.MINOR.PATCH` untuk mempublikasikan image multi-arsitektur ke GitHub Container Registry (`ghcr.io`), misalnya:

```bash
git tag v1.2.3
git push origin v1.2.3
```

Workflow mempublikasikan `ghcr.io/devoverid/siti:1.2.3`, `:1.2`, dan `:latest`. Push ke `master` tidak mempublikasikan image.

## Status & Pengembangan Lanjutan

- [x] Telegram bot
- [x] HTTP API (session/chat/SSE/knowledge, auth & rate limit dasar)
- [x] WhatsApp adapter (Meta Cloud API webhook, implementasi awal)
- [x] Percakapan generik, knowledge/RAG ringan, tool registry/plugin dasar, logging run
- [ ] Streaming token penuh dari provider LLM
- [ ] Generalisasi identitas/idempotency lintas-channel end-to-end
- [ ] RAG lanjutan (embedding/vector search), workflow/goal persisten, guardrails & policy tool
- [ ] Auth multi-tenant/kuota terdistribusi, observability terstruktur, dukungan media & hardening WhatsApp

## License

MIT
