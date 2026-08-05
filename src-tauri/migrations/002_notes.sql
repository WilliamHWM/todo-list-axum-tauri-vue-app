CREATE TABLE IF NOT EXISTS notes (
                                     id TEXT PRIMARY KEY,
                                     task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
                                     content TEXT NOT NULL,
                                     created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
CREATE INDEX IF NOT EXISTS idx_notes_task_id ON notes(task_id);
