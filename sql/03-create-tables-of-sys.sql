CREATE TABLE IF NOT EXISTS `sys_sitemap` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `loc` text,
  `freq` varchar(100),
  `priority` float
);

CREATE TABLE IF NOT EXISTS `sys_articlemap` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(512),
  `loc` text,
  `title` text,
  `language` varchar(100),
  `keywords` text
);

CREATE TABLE IF NOT EXISTS `sys_filemap` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `src` varchar(2048) NOT NULL,
  `dest` varchar(2048) NOT NULL,
  INDEX `idx_tenant_id` (`tenant_id`)
);

CREATE TABLE IF NOT EXISTS `sys_tenant` (
  `host` varchar(200) PRIMARY KEY,
  `id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `sys_api_map` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `mode` integer,
  `name` varchar(200),
  `url` varchar(1000),
  `parser` json,

  UNIQUE KEY `unique_tenant_name_mode` (`tenant_id`, `name`, `mode`)
);

CREATE TABLE IF NOT EXISTS `sys_database_map` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `token` varchar(200),

  UNIQUE KEY `unique_tenant` (`tenant_id`)
);

CREATE TABLE IF NOT EXISTS `sys_table_map` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `backend` integer,
  `name` varchar(200),
  `schema` json,

  UNIQUE KEY `unique_tenant_name_mode` (`tenant_id`, `name`)
);

CREATE TABLE IF NOT EXISTS `sys_token_map` (
  `id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` BIGINT,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `service` VARCHAR(200) NOT NULL,
  `token` VARBINARY(1024) NOT NULL,

  UNIQUE KEY `uk_tenant_service` (`tenant_id`, `service`)
);
