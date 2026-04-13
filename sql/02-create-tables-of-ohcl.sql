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
  `id` integer PRIMARY KEY AUTO_INCREMENT, -- Link directly to ohcl_mapping_product_in_store_to_symbol.id
  `buy` float NOT NULL DEFAULT '0',
  `sell` float NOT NULL DEFAULT '0',
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `ohcl_price_history` (
  `id` bigint PRIMARY KEY AUTO_INCREMENT,
  `symbol_id` integer NOT NULL, -- Link directly to ohcl_mapping_product_in_store_to_symbol.id
  `buy` float NOT NULL DEFAULT '0',
  `sell` float NOT NULL DEFAULT '0',
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `ohcl_mapping_product_in_store_to_symbol` (
  `id` integer PRIMARY KEY NOT NULL,
  `symbol` integer, -- Link directly to ohcl_symbols.id
  `store` integer NOT NULL, -- Link directly to ohcl_store_locations.id
  `product_name` varchar(500) NOT NULL,
  `scope` int NOT NULL DEFAULT '0',
  `location` int,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `ohcl_stores` (
  `id` integer PRIMARY KEY NOT NULL,
  `name` varchar(500) NOT NULL,
  `website` varchar(200),
  `phone` varchar(40),
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  UNIQUE KEY `unique_name` (`name`)
);

CREATE TABLE IF NOT EXISTS `ohcl_store_locations` (
  `id` integer PRIMARY KEY NOT NULL,
  `store` integer NOT NULL,
  `address_line` varchar(500),
  `district` varchar(100),
  `province` varchar(100),
  `latitude` float NOT NULL DEFAULT '0',
  `longitude` float NOT NULL DEFAULT '0',
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  UNIQUE KEY `unique_address` (`address_line`, `district`, `province`)
);

ALTER TABLE ohcl_stores CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
ALTER TABLE ohcl_store_locations CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
ALTER TABLE ohcl_mapping_product_in_store_to_symbol CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
