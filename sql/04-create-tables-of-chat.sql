CREATE TABLE IF NOT EXISTS `chat_threads` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `tenant_id` integer,
  `thread_id` varchar(100),
  `source_id` varchar(100),
  `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  `updated_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `status` integer,
  `source_type` integer
);
