# siti_whatsapp

Adapter channel **WhatsApp Cloud API (Meta)** untuk Siti.

## Fitur
- Webhook `GET/POST /webhook`: verifikasi subscription dan penerimaan pesan teks.
- Memproses pesan via `siti_core::Agent`, menyimpan percakapan (tabel generik `conversations`), dan mengirim balasan lewat Graph API.
- Jika kredensial kirim belum lengkap, pesan tetap diproses (balasan dicatat di log).

## Konfigurasi
Lihat `.env.example`:
- `WHATSAPP_VERIFY_TOKEN` (verifikasi webhook)
- `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_GRAPH_VERSION`
- Konfigurasi core/LLM/DB sama seperti service lain.

## Jalankan
```bash
docker compose up -d db
cargo run -p siti_whatsapp
```
Expose endpoint `/webhook` ke URL publik dan konfigurasikan di Meta for Developers (WhatsApp > Configuration).