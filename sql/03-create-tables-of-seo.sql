CREATE TABLE IF NOT EXISTS `seo_sitemap` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `loc` text,
  `freq` varchar(100),
  `priority` float
);

CREATE TABLE IF NOT EXISTS `seo_articlemap` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `name` varchar(512),
  `loc` text,
  `title` text,
  `language` varchar(100),
  `keywords` text,
);

CREATE TABLE IF NOT EXISTS `seo_tenant` (
  `host` varchar(200) PRIMARY KEY,
  `id` integer,
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
