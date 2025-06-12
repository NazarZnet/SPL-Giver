-- Groups table
CREATE TABLE IF NOT EXISTS groups (
    id INTEGER PRIMARY KEY NOT NULL ,
    spl_share_percent REAL NOT NULL,
    spl_total REAL NOT NULL,
    spl_price REAL NOT NULL,
    initial_unlock_percent REAL NOT NULL,
    unlock_interval_seconds INTEGER NOT NULL,
    unlock_percent_per_interval REAL NOT NULL,
    unlock_task_spawned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);