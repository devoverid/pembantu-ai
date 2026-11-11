# siti_api

HTTP API untuk menjalankan Siti sebagai **chatbot & agent** engine.

## Endpoint
- `GET /health`
- `POST /v1/sessions`, `GET /v1/sessions`, `GET /v1/sessions/{id}`
- `POST /v1/knowledge`, `GET /v1/knowledge?q=...`, `DELETE /v1/knowledge/{id}`
- `POST /v1/chat` (JSON)
- `POST /v1/chat/stream` (SSE: `start`, `tool_started`, `final`, `error`, `done`)

## Auth & rate limit
- Opsional global key: set `API_KEY` dan kirim via `X-API-Key` atau `Authorization: Bearer <key>`.
- `REQUIRE_AUTH=true` untuk mewajibkan key.
- `RATE_LIMIT_PER_MINUTE` (in-memory, per key/IP bucket).

## Jalankan
```bash
docker compose up -d db
cp crates/siti_api/.env.example .env # atau set env manual
cargo run -p siti_api
```

Lihat `siti/PENGEMBANGAN.md` untuk arsitektur dan roadmap.