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
)
