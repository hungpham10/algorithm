
INSERT IGNORE INTO `ohcl_mapping_broker_resolution` (`id`, `broker_id`, `resolution_id`, `resolution`) VALUES
(1, 1, 1, '1D'),
(2, 1, 2, '1H'),
(3, 1, 3, '1W');

INSERT IGNORE INTO `ohcl_mapping_broker_resolution` (`id`, `broker_id`, `resolution_id`, `resolution`) VALUES
(1, 1, 1, '1D'),
(2, 1, 2, '1H'),
(3, 1, 3, '1W'),
(4, 3, 1, '1D'),
(5, 3, 2, '1H'),
(6, 3, 3, '1W'),
(7, 4, 1, '1D'),
(8, 4, 2, '1H'),
(9, 4, 3, '1W'),
(10, 5, 1, '1D'),
(11, 5, 2, '1H'),
(12, 5, 3, '1W'),
(13, 6, 1, '1D'),
(14, 6, 2, '1H'),
(15, 6, 3, '1W'),
(16, 7, 1, '1D'),
(17, 7, 2, '1H'),
(18, 7, 3, '1W'),
(19, 8, 1, '1D'),
(20, 8, 2, '1H'),
(21, 8, 3, '1W');

INSERT IGNORE INTO `ohcl_brokers` (`id`, `name`) VALUES
(1, 'dnse'),
(2, 'binance'),
(3, 'ssi'),
(4, 'vix'),
(5, 'dragon'),
(6, 'yahoo'),
(7, 'msn');

INSERT IGNORE INTO `ohcl_products` (`id`, `broker_id`, `name`, `enabled`) VALUES
(1, 1, 'vn30', 1),
(2, 1, 'vn100', 1),
(3, 1, 'future', 1),
(4, 1, 'etf', 1),
(5, 1, 'cw', 1),
(6, 3, 'vn30', 1),
(7, 3, 'vn100', 1),
(8, 3, 'future', 1),
(9, 3, 'etf', 1),
(10, 3, 'cw', 1),
(11, 4, 'vn30', 1),
(12, 4, 'vn100', 1),
(13, 4, 'future', 1),
(14, 4, 'etf', 1),
(15, 4, 'cw', 1),
(16, 5, 'vn30', 1),
(17, 5, 'vn100', 1),
(18, 5, 'future', 1),
(19, 5, 'etf', 1),
(20, 5, 'cw', 1),
(36, 2, 'spot', 1),
(37, 2, 'future', 1);

INSERT IGNORE INTO `ohcl_resolution` (`id`, `resolution`) VALUES
(1, '1D'),
(2, '1H'),
(3, '1W');
