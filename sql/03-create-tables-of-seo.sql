CREATE TABLE IF NOT EXISTS `seo_sitemap` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now()),
  `loc` text,
  `freq` varchar(100),
  `priority` float
);

CREATE TABLE IF NOT EXISTS `seo_tenant` (
  `host` varchar(200) PRIMARY KEY,
  `id` integer,
  `created_at` timestamp DEFAULT (now()),
  `updated_at` timestamp DEFAULT (now())
);
