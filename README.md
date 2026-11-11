<h1 align="center">Siti</h1>
<p align="center">A telegram bot that can talk and help you</p>

<p align="center">
You can try the bot here: <a href="https://t.me/PembantuAI_Bot">@PembantuAI_Bot</a>
</p>


<p align="center">
  <img align="center" src="https://github.com/devoverid/pembantu-ai/assets/45036724/f6a0e236-f816-4402-91de-7ab467d43573">
</p>


## TODO

- [x] Telegram bot
- [ ] Run Bahasa Indonesia models
- [ ] Learn context
- [ ] Add Whatsapp port

## Run locally

1. Clone the repository
```sh
git clone https://github.com/devoverid/pembantu-ai/
cd pembantu-ai
```
2. Clone .env file
```
cp .env.example .env
```
3. Set up your values in .env (see `.env.example`)
```
TELOXIDE_TOKEN=
PROVIDER_TEXT_GENERATION=openrouter
MODEL_TEXT_GENERATION=google/gemini-3.5-flash-lite
OPENROUTER_API_KEY=
GEMINI_API_KEY=
```
   For group chats, disable privacy mode in @BotFather (`/setprivacy` → Disable) so the bot can read group messages.
4. Start Postgres (set `POSTGRES_PORT` and `DATABASE_URL` if 5432 is taken)
```
docker compose up -d db
```
5. Run the telegram bot
```
cargo run -p siti_telegram
```

## Run with the published Docker image

`docker-compose.prod.yml` runs the prebuilt `zuramai/pembantu-ai:1.1.0` image from Docker Hub with Postgres, with no local build needed.

1. Create `.env` from `.env.example` and fill in your values (see step 3 above). `DATABASE_URL` is set by the compose file.
```sh
cp .env.example .env
```
2. Pull and start the stack
```sh
docker compose -f docker-compose.prod.yml up -d
```
3. Follow the bot logs
```sh
docker compose -f docker-compose.prod.yml logs -f bot
```
4. Stop it
```sh
docker compose -f docker-compose.prod.yml down
```

To upgrade, change the `image` tag in `docker-compose.prod.yml`, then run `docker compose -f docker-compose.prod.yml pull && docker compose -f docker-compose.prod.yml up -d`.

## Docker image releases

Push a `vMAJOR.MINOR.PATCH` tag to publish a multi-architecture image to Docker Hub. For example:

```sh
git tag v1.2.3
git push origin v1.2.3
```

The workflow publishes `zuramai/pembantu-ai:1.2.3`, `:1.2`, and `:latest`. Pushes to `master` do not publish images.

## License 
MIT
