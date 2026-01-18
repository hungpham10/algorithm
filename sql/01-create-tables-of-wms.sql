-- BEGIN: danh sách các bảng liên quan inventory và tồn kho
CREATE TABLE IF NOT EXISTS `wms_stocks`   (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `name` varchar(255) NOT NULL,
  `unit` varchar(50) NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY `unique_tenant_name` (`tenant_id`, `name`)
);

CREATE TABLE IF NOT EXISTS `wms_contents` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `uname` varchar(255) NOT NULL,
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
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY `unique_tenant_lot_number` (`tenant_id`, `lot_number`)
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
  `name` varchar(255) NOT NULL,
  `publish` BOOLEAN DEFAULT FALSE,
  `description` varchar(255),
  `node` INTEGER DEFAULT 0,
  `zone` INTEGER DEFAULT 0,
  `is_left` BOOLEAN DEFAULT FALSE,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY `unique_tenant_name` (`tenant_id`, `name`)
);

CREATE TABLE IF NOT EXISTS  `wms_stock_shelves` (
  `id` integer AUTO_INCREMENT PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `shelf_id` integer NOT NULL,
  `quantity` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS  `wms_sales` (
  `id` integer AUTO_INCREMENT PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `order_id` integer NOT NULL,
  `cost_price` DOUBLE NOT NULL,
  `status` integer DEFAULT 0,
  `version` integer DEFAULT 0,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY `unique_tenant_order_id` (`tenant_id`, `order_id`)
);

CREATE TABLE IF NOT EXISTS `wms_sale_events` (
  `id` integer AUTO_INCREMENT PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `sale_id` integer NOT NULL,
  `stock_id` integer,
  `version` integer NOT NULL,
  `status` integer DEFAULT 0,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY `unique_tenant_sale_id` (`tenant_id`, `sale_id`, `version`)
);

CREATE TABLE IF NOT EXISTS  `wms_items` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `lot_id` integer NOT NULL,
  `shelf_id` integer,
  `order_id` integer,
  `assigned_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `expired_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `status` integer DEFAULT 0,
  `cost_price` DOUBLE NOT NULL,
  `barcode` varchar(255)

  UNIQUE KEY `unique_tenant_barcode` (`tenant_id`, `barcode`)
);
-- END

-- BEGIN: danh sách các bảng liên quan topology
CREATE TABLE `wms_zones` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(255) NOT NULL,
  `description` text,
  `pos_x` float NOT NULL DEFAULT 0,
  `pos_y` float NOT NULL DEFAULT 0,
  `height` float NOT NULL,
  `width` float NOT NULL,

  UNIQUE KEY `unique_tenant_name` (`tenant_id`, `name`)
);

CREATE TABLE `wms_nodes` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `zone_id` integer,
  `name` varchar(255),
  `kind` integer NOT NULL DEFAULT 0,
  `pos_x` float NOT NULL DEFAULT 0,
  `pos_y` float NOT NULL DEFAULT 0,

  UNIQUE KEY `unique_tenant_name` (`tenant_id`, `name`)
);

CREATE TABLE `wms_paths` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `from_node_id` integer,
  `to_node_id` integer,
  `name` varchar(50),
  `zone_id` integer,
  `distance` float,
  `is_one_way` boolean DEFAULT false,
  `sharps` json,
  `status` integer NOT NULL DEFAULT 0,

  UNIQUE KEY `unique_tenant_name` (`tenant_id`, `name`)
);
-- END

-- BEGIN: danh sách các bảng liên quan feature picking
CREATE TABLE `wms_picking_plans` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `version` integer DEFAULT 0,
  `status` integer
);

CREATE TABLE `wms_picking_plans_in_zones` (
  `id` integer PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `plan_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
);

CREATE TABLE `wms_picking_plans_in_nodes` (
  `id` integer PRIMARY KEY,
  `tenant_id` integer NOT NULL,
  `plan_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE `wms_picking_routes` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `tenant_id` integer NOT NULL,
  `depend_id` integer,
  `picking_id` integer,
  `status` integer,
  `version` integer DEFAULT 0,
  `paths` json
);

CREATE TABLE `wms_picking_goods` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `sale_id` integer NOT NULL,
  `plan_id` integer NOT NULL,
  `route_id` integer,
  `event_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `status` integer,
  `is_ready_to_pack` boolean,

  UNIQUE KEY `unique_tenant_sale` (`tenant_id`, `sale_id`)
);

CREATE TABLE `wms_picking_plan_events` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `plan_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `version` integer NOT NULL,
  `status` integer NOT NULL,

  UNIQUE KEY `unique_tenant_plan_status` (`tenant_id`, `plan_id`, `version`)
);

CREATE TABLE `wms_picking_route_events` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `version` integer NOT NULL,
  `actor_id` integer NOT NULL,
  `route_id` integer NOT NULL,
  `status` integer NOT NULL,

  UNIQUE KEY `unique_tenant_route_status` (`tenant_id`, `route_id`, `version`)
);

CREATE TABLE `wms_picking_items` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `item_id` integer,
  `event_id` integer,
  `ledger_id` integer,

  UNIQUE KEY `unique_tenant_item` (`tenant_id`, `item_id`)
);
-- END

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

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_picking_plans_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_picking_plans ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_picking_plans DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_picking_routes_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_picking_routes ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_picking_routes DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_picking_goods_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_picking_goods ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_picking_goods DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_picking_events_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_picking_events ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_picking_events DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;

DELIMITER $$
CREATE EVENT IF NOT EXISTS manage_wms_picking_items_partitions
ON SCHEDULE EVERY 1 MONTH
STARTS CURRENT_TIMESTAMP
DO
BEGIN
    DECLARE next_month INT;
    DECLARE old_month INT;

    -- 1️⃣ Tháng tiếp theo: thêm partition mới
    SET next_month = DATE_FORMAT(DATE_ADD(CURDATE(), INTERVAL 1 MONTH), '%Y%m');
    SET @sql_add = CONCAT('ALTER TABLE wms_picking_items ADD PARTITION (PARTITION p', next_month,
                          ' VALUES LESS THAN (', next_month + 1, '))');
    PREPARE stmt_add FROM @sql_add;
    EXECUTE stmt_add;
    DEALLOCATE PREPARE stmt_add;

    -- 2️⃣ Tháng quá hạn (>6 tháng): drop partition cũ
    SET old_month = DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 MONTH), '%Y%m');
    SET @sql_drop = CONCAT('ALTER TABLE wms_picking_items DROP PARTITION IF EXISTS p', old_month);
    PREPARE stmt_drop FROM @sql_drop;
    EXECUTE stmt_drop;
    DEALLOCATE PREPARE stmt_drop;
END$$
DELIMITER ;
