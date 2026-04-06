CREATE TABLE IF NOT EXISTS `ohcl_products` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(500) NOT NULL,
  `enabled` boolean DEFAULT FALSE,

  UNIQUE KEY `unique_name` (`name`)
);

CREATE TABLE IF NOT EXISTS `ohcl_brokers` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(100) NOT NULL,
  `alias` integer DEFAULT NULL,

  UNIQUE KEY `unique_name` (`name`)
);

CREATE TABLE IF NOT EXISTS `ohcl_resolution` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `resolution` varchar(5) NOT NULL,

  UNIQUE KEY `unique_resolution` (`resolution`)
);

CREATE TABLE IF NOT EXISTS `ohcl_mapping_broker_resolution` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `resolution_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `resolution` varchar(5) NOT NULL,

  UNIQUE KEY `unique_broker_resolution` (`broker_id`, `resolution_id`)
);

CREATE TABLE IF NOT EXISTS `ohcl_broker_limitation` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  -- Limit how anonymous users can see data
  `guest_max_history_days` integer DEFAULT 7,
  `guest_invisible` boolean DEFAULT FALSE,

  UNIQUE KEY `unique_broker` (`broker_id`)
);

CREATE TABLE IF NOT EXISTS `ohcl_symbols` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer NOT NULL,
  `product_id` integer NOT NULL,
  `name` varchar(255) NOT NULL,
  `symbol` varchar(50) NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  UNIQUE KEY `unique_symbol_per_broker` (`broker_id`, `product_id`, `symbol`)
);

CREATE TABLE IF NOT EXISTS `ohcl_price_current` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `buy` decimal(20, 2) DEFAULT 0,
  `sell` decimal(20, 2) DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `ohcl_price_history` (
  `id` bigint PRIMARY KEY AUTO_INCREMENT,
  `symbol_id` integer NOT NULL,
  `buy` decimal(20, 2) NOT NULL,
  `sell` decimal(20, 2) NOT NULL,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `ohcl_store_locations` (
  `id` integer NOT NULL,

  `address_line` varchar(500) NOT NULL,
  `district` varchar(100) NOT NULL,
  `province` varchar(100) NOT NULL,

  `latitude` decimal(10, 8) NOT NULL,
  `longitude` decimal(11, 8) NOT NULL,

  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
