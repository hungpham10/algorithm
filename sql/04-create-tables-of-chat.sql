CREATE TABLE IF NOT EXISTS `chat_threads` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `thread_id` varchar(100),
  `source_id` varchar(100),
  `source_type` integer
);

CREATE TABLE IF NOT EXISTS `chat_messages` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `thread_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `partition_at` INT AS (YEAR(created_at)*100 + MONTH(created_at)) STORED,
  `message` text
) PARTITION BY RANGE (partition_at) (
  PARTITION p202509 VALUES LESS THAN (202510),
  PARTITION p202510 VALUES LESS THAN (202511),
  PARTITION p202511 VALUES LESS THAN (202512),
  PARTITION p202512 VALUES LESS THAN (202513),
  PARTITION pMax VALUES LESS THAN MAXVALUE
);

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_chat_messages
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE chat_messages ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE chat_messages DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;
