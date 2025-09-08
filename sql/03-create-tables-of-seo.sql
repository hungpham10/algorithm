CREATE TABLE IF NOT EXISTS `seo_sitemap` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `host` varchar(200),
  `created_at` datetime,
  `updated_at` datetime,
  `loc` text,
  `freq` varchar(100),
  `priority` float
);
