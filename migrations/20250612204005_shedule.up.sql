-- Schedule table for MySQL
CREATE TABLE IF NOT EXISTS `schedule` (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    group_id BIGINT  NOT NULL,
    buyer_wallet VARCHAR(50) NOT NULL,
    scheduled_at DATETIME NOT NULL,
    amount_lamports BIGINT UNSIGNED NOT NULL,
    percent DOUBLE NOT NULL DEFAULT 0.0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'success', 'failed'
    error_message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES `groups`(id) ON DELETE CASCADE,
    FOREIGN KEY (buyer_wallet) REFERENCES `buyers`(wallet) ON DELETE CASCADE
);