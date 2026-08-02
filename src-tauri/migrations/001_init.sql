CREATE TABLE IF NOT EXISTS tasks (
                                     id TEXT PRIMARY KEY,
                                     title TEXT NOT NULL,
                                     completed BOOLEAN NOT NULL DEFAULT 0,
                                     created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );