CREATE TABLE IF NOT EXISTS `chat_threads` (
  `id` integer PRIMARY KEY AUTO_INCREMENT,
  `thread_id` varchar(100),
  `source_id` varchar(100),
  `status` integer,
  `source_type` integer
);
