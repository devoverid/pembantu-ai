CREATE TABLE scheduled_tasks (
    id BIGSERIAL PRIMARY KEY,
    chat_id BIGINT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    thread_id INTEGER,
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('reminder', 'agent_task')),
    label TEXT,
    payload TEXT NOT NULL,
    schedule JSONB NOT NULL,
    timezone TEXT NOT NULL,
    next_run_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'done', 'cancelled', 'failed')),
    last_run_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX scheduled_tasks_due_idx ON scheduled_tasks (next_run_at) WHERE status = 'active';
