CREATE TABLE IF NOT EXISTS `ohcl_products` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now()),
  `name` varchar(500) NOT NULL,
  `enabled` boolean
);

CREATE TABLE IF NOT EXISTS `ohcl_brokers` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now()),
  `name` varchar(100) NOT NULL
);

CREATE TABLE IF NOT EXISTS `ohcl_resolution` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now()),
  `resolution` varchar(5) NOT NULL
);

CREATE TABLE IF NOT EXISTS `ohcl_mapping_broker_resolution` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `broker_id` integer,
  `resolution_id` integer,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now()),
  `resolution` varchar(5) NOT NULL
);
