-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    buyer_wallet TEXT NOT NULL REFERENCES buyers(wallet) ON DELETE CASCADE,
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    amount REAL NOT NULL,
    percent REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL, -- "pending", "success", "failed"
    error_message TEXT,
    sent_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);