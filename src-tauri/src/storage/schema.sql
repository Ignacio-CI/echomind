CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    locale TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transcripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    t_start REAL NOT NULL,
    t_end REAL NOT NULL,
    text TEXT NOT NULL,
    is_final INTEGER NOT NULL,
    FOREIGN KEY(meeting_id) REFERENCES meetings(id)
);

CREATE TABLE IF NOT EXISTS summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    summary TEXT NOT NULL,
    action_items_json TEXT,
    FOREIGN KEY(meeting_id) REFERENCES meetings(id)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
