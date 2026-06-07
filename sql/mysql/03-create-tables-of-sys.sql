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
  `host` VARCHAR(200) PRIMARY KEY,
  `id` BIGINT NOT NULL UNIQUE,
  `jwt_mode` VARCHAR(20) DEFAULT NULL,
  `jwt_secret` BIGINT DEFAULT NULL COMMENT 'Trỏ tới sys_token_map.id',
  `oidc_jwks_url` VARCHAR(500) DEFAULT NULL,
  `oidc_issuer` VARCHAR(255) DEFAULT NULL,
  `oidc_client_id` VARCHAR(255) DEFAULT NULL COMMENT 'Client ID công khai (chuỗi)',
  `oidc_client_secret` BIGINT DEFAULT NULL COMMENT 'Trỏ tới sys_token_map.id',
  `oidc_expected_alg` VARCHAR(10) DEFAULT NULL,
  `session_secret` BIGINT DEFAULT NULL COMMENT 'Trỏ tới sys_token_map.id',
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
  `ttl` integer,

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

CREATE TABLE IF NOT EXISTS `sys_streams` (
	`id` BIGINT PRIMARY KEY AUTO_INCREMENT,
  	`tenant_id` BIGINT NOT NULL,
  	`context` json NOT NULL,
	`created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `sys_sinks` (
	`id` BIGINT PRIMARY KEY AUTO_INCREMENT,
	`tenant_id` BIGINT NOT NULL,
	`handler` json NOT NULL,
	`created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS `sys_link_streams_to_sinks` (
	`id` BIGINT PRIMARY KEY AUTO_INCREMENT,
	`tenant_id` BIGINT NOT NULL,
	`sink_id` BIGINT NOT NULL,
	`stream_id` BIGINT NOT NULL,
	`enabled` BOOLEAN DEFAULT FALSE,
	`created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
