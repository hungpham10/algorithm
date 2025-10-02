CREATE TABLE IF NOT EXISTS `wms_stocks`   (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `name` varchar(255) NOT NULL,
  `unit` varchar(50) NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS  `wms_lots` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `lot_number` varchar(255) NOT NULL,
  `quantity` integer NOT NULL DEFAULT 0,
  `supplier` varchar(255),
  `entry_date` timestamp NOT NULL DEFAULT (now()),
  `cost_price` DOUBLE,
  `status` integer DEFAULT 0,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS  `wms_stock_entries` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `lot_id` integer NOT NULL,
  `quantity` integer NOT NULL,
  `status` integer NOT NULL,
  `expired_at` TIMESTAMP,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS  `wms_shelves` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `name` varchar(255) UNIQUE NOT NULL,
  `publish` BOOLEAN DEFAULT FALSE,
  `description` varchar(255),
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `wms_stock_lots` (
  `id` integer AUTO_INCREMENT PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `lot_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS  `wms_stock_shelves` (
  `id` integer AUTO_INCREMENT PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `shelf_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
);

CREATE TABLE IF NOT EXISTS  `wms_sales` (
  `id` integer AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `order_id` integer NOT NULL,
  `cost_price` DOUBLE NOT NULL,
  `status` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `partition_at` integer AS (YEAR(created_at)*100 + MONTH(created_at)) STORED,
  PRIMARY KEY (id, partition_at)
) PARTITION BY RANGE (partition_at) (
  PARTITION p202509 VALUES LESS THAN (202510),
  PARTITION p202510 VALUES LESS THAN (202511),
  PARTITION p202511 VALUES LESS THAN (202512),
  PARTITION p202512 VALUES LESS THAN (202513),
  PARTITION pMax VALUES LESS THAN MAXVALUE
);

CREATE TABLE IF NOT EXISTS `wms_sale_events` (
  `id` integer AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `sale_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `status` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `partition_at` integer AS (YEAR(created_at)*100 + MONTH(created_at)) STORED,
  PRIMARY KEY (id, partition_at)
) PARTITION BY RANGE (partition_at) (
  PARTITION p202509 VALUES LESS THAN (202510),
  PARTITION p202510 VALUES LESS THAN (202511),
  PARTITION p202511 VALUES LESS THAN (202512),
  PARTITION p202512 VALUES LESS THAN (202513),
  PARTITION pMax VALUES LESS THAN MAXVALUE
);

CREATE TABLE IF NOT EXISTS  `wms_items` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `lot_id` integer NOT NULL,
  `shelf_id` integer NOT NULL,
  `order_id` integer NOT NULL,
  `assigned_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `expired_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `status` integer NOT NULL,
  `cost_price` DOUBLE NOT NULL,
  `barcode` varchar(255)
);

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_sales_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_sales ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_sales DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;


DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_sale_events_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_sale_events ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_sale_events DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;
