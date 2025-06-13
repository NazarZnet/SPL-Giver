-- Add up migration script here
CREATE TABLE IF NOT EXISTS schedule (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    buyer_wallet TEXT NOT NULL REFERENCES buyers(wallet) ON DELETE CASCADE,
    scheduled_at DATETIME NOT NULL,
    amount REAL NOT NULL,
    percent REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending',  'success', 'failed'
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);