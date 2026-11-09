CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    username TEXT,
    full_name TEXT NOT NULL,
    language_code TEXT,
    timezone TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE chats (
    id BIGINT PRIMARY KEY,
    kind TEXT NOT NULL,
    title TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE messages (
    id BIGSERIAL PRIMARY KEY,
    chat_id BIGINT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    telegram_message_id INTEGER NOT NULL,
    thread_id INTEGER,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    text TEXT NOT NULL,
    reply_to_telegram_message_id INTEGER,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE (chat_id, telegram_message_id)
);
CREATE INDEX messages_chat_created_idx ON messages (chat_id, created_at DESC);

CREATE TABLE memories (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX memories_user_idx ON memories (user_id, created_at DESC);
