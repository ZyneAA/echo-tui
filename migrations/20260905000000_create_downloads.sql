CREATE TABLE IF NOT EXISTS downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'youtube',
    status TEXT NOT NULL DEFAULT 'running',
    file_path TEXT,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME
);
