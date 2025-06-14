-- Buyers table
CREATE TABLE IF NOT EXISTS buyers (
    wallet TEXT PRIMARY KEY NOT NULL,
    paid_sol REAL NOT NULL,
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    received_percent REAL NOT NULL DEFAULT 0.0,
    received_spl REAL NOT NULL DEFAULT 0,
    pending_spl REAL NOT NULL DEFAULT 0,
    error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);