CREATE TABLE IF NOT EXISTS `wms_stocks`   (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `name` varchar(255) NOT NULL,
  `quantity` integer NOT NULL DEFAULT 0,
  `unit` varchar(50) NOT NULL,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);

CREATE TABLE IF NOT EXISTS  `wms_lots` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer NOT NULL,
  `stock_id` integer NOT NULL,
  `lot_number` varchar(255) NOT NULL,
  `quantity` integer NOT NULL DEFAULT 0,
  `supplier` varchar(255),
  `entry_date` timestamp NOT NULL DEFAULT (now()),
  `cost_price` decimal(10,2),
  `status` varchar(50) DEFAULT 'available',
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);

CREATE TABLE IF NOT EXISTS  `wms_stock_entries` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `lot_id` integer NOT NULL,
  `quantity` integer NOT NULL,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);

CREATE TABLE IF NOT EXISTS  `wms_shelves` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `name` varchar(255) UNIQUE NOT NULL,
  `description` varchar(255),
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);

CREATE TABLE IF NOT EXISTS  `wms_stock_shelves` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `lot_id` integer,
  `item_id` integer,
  `shelf_id` integer,
  `assigned_at` timestamp DEFAULT (now()),
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);

CREATE TABLE IF NOT EXISTS  `wms_sales` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `order_id` integer,
  `item_id` integer,
  `cost_price` decimal(10,2) NOT NULL,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);

CREATE TABLE IF NOT EXISTS  `wms_items` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `stock_id` integer,
  `lot_id` integer,
  `sale_id` integer,
  `expired_at` timestamp DEFAULT (now()),
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now()),
  `cost_price` decimal(10,2) NOT NULL,
  `barcode` varchar(255)
);
