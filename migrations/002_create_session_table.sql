-- Create session table for photo booth sessions
CREATE TABLE IF NOT EXISTS session (
    id TEXT PRIMARY KEY NOT NULL,
    group_name TEXT NULL,
    created_at TEXT NOT NULL,
    weapon INTEGER NULL,
    land INTEGER NULL,
    companion INTEGER NULL,
    email TEXT NULL,
    photo_path TEXT NULL,
    copies_printed INTEGER NOT NULL DEFAULT 0,
    story_text TEXT NULL,
    headline TEXT NULL
);

-- Create index on created_at for chronological queries
CREATE INDEX IF NOT EXISTS idx_session_created_at ON session(created_at);

-- Create index on email for lookup queries
CREATE INDEX IF NOT EXISTS idx_session_email ON session(email);
