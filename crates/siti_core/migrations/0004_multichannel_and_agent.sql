-- Multi-channel foundation, API sessions, auth, and knowledge (additive, keeps Telegram-compatible schema).

-- Conversations/sessions for channel-agnostic chat (API, WhatsApp, etc.)
CREATE TABLE IF NOT EXISTS conversations (
    id BIGSERIAL PRIMARY KEY,
    channel TEXT NOT NULL, -- e.g. telegram, api, whatsapp
    external_id TEXT NOT NULL, -- channel-native conversation/chat id
    title TEXT,
    kind TEXT NOT NULL DEFAULT 'private',
    persona TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (channel, external_id)
);

CREATE TABLE IF NOT EXISTS conversation_participants (
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_external_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    username TEXT,
    language_code TEXT,
    timezone TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (conversation_id, user_external_id)
);

CREATE TABLE IF NOT EXISTS conversation_messages (
    id BIGSERIAL PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    external_message_id TEXT,
    user_external_id TEXT,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    text TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (conversation_id, external_message_id)
);
CREATE INDEX IF NOT EXISTS conversation_messages_conv_created_idx
    ON conversation_messages (conversation_id, created_at DESC, id DESC);

-- Simple API keys for HTTP API auth (hashed tokens recommended in production; this stores token as-is for scaffold)
CREATE TABLE IF NOT EXISTS api_keys (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    scopes TEXT[] NOT NULL DEFAULT ARRAY['chat'],
    rate_limit_per_minute INTEGER NOT NULL DEFAULT 60,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

-- Knowledge base documents for lightweight RAG/retrieval
CREATE TABLE IF NOT EXISTS knowledge_documents (
    id BIGSERIAL PRIMARY KEY,
    owner_user_id BIGINT, -- optional scope (matches users.id when applicable)
    conversation_id BIGINT REFERENCES conversations(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS knowledge_documents_owner_idx ON knowledge_documents (owner_user_id);
CREATE INDEX IF NOT EXISTS knowledge_documents_conv_idx ON knowledge_documents (conversation_id);
-- basic text search index
CREATE INDEX IF NOT EXISTS knowledge_documents_content_tsv_idx
    ON knowledge_documents USING GIN (to_tsvector('simple', coalesce(title,'') || ' ' || coalesce(content,'')));

-- Agent run logs for observability
CREATE TABLE IF NOT EXISTS agent_runs (
    id BIGSERIAL PRIMARY KEY,
    channel TEXT NOT NULL,
    conversation_ref TEXT NOT NULL,
    user_ref TEXT,
    input_text TEXT NOT NULL,
    output_text TEXT,
    tools_used TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    steps INTEGER NOT NULL DEFAULT 0,
    success BOOLEAN NOT NULL DEFAULT false,
    error TEXT,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS agent_runs_created_idx ON agent_runs (created_at DESC);
