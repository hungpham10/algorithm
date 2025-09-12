CREATE TABLE IF NOT EXISTS `ohcl_products` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(500) NOT NULL,
  `enabled` boolean
);

CREATE TABLE IF NOT EXISTS `ohcl_brokers` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(100) NOT NULL
);

CREATE TABLE IF NOT EXISTS `ohcl_resolution` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `resolution` varchar(5) NOT NULL
);

CREATE TABLE IF NOT EXISTS `ohcl_mapping_broker_resolution` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `resolution_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `resolution` varchar(5) NOT NULL
);
