-- Transactions table for MySQL
CREATE TABLE IF NOT EXISTS `transactions` (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    buyer_wallet VARCHAR(50) NOT NULL,
    group_id BIGINT NOT NULL,
    amount_lamports BIGINT UNSIGNED NOT NULL,
    percent DOUBLE NOT NULL DEFAULT 0.0,
    status VARCHAR(20) NOT NULL, -- "success", "failed"
    error_message TEXT,
    sent_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (buyer_wallet) REFERENCES `buyers`(wallet) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES `groups`(id) ON DELETE CASCADE
);