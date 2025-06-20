-- Groups table for MySQL
CREATE TABLE IF NOT EXISTS `groups` (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    spl_share_percent DOUBLE NOT NULL,
    spl_total_lamports BIGINT UNSIGNED NOT NULL,
    spl_price_lamports BIGINT UNSIGNED NOT NULL, 
    initial_unlock_percent DOUBLE NOT NULL,
    unlock_interval_seconds BIGINT NOT NULL,
    unlock_percent_per_interval DOUBLE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);