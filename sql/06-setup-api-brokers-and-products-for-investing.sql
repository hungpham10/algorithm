
INSERT IGNORE INTO `ohcl_mapping_broker_resolution` (`id`, `broker_id`, `resolution_id`, `resolution`) VALUES
(1, 1, 1, '1D'),
(2, 1, 2, '1H'),
(3, 1, 3, '4H'),
(4, 1, 4, '1M'),
(5, 1, 5, '5m'),
(6, 1, 6, '1m'),
(6, 1, 7, '1W'),
(7, 2, 1, '1D'),
(8, 2, 2, '1H'),
(9, 2, 3, '4H'),
(10, 2, 4, '1M'),
(11, 2, 5, '5m'),
(12, 2, 6, '1m'),
(13, 2, 7, '1W'),
(14, 3, 1, '1D'),
(15, 3, 2, '1H'),
(16, 3, 3, '4H'),
(17, 3, 4, '1M'),
(18, 3, 5, '5m'),
(19, 3, 6, '1m'),
(20, 3, 7, '1W'),
(21, 4, 1, '1D'),
(22, 4, 2, '1H'),
(23, 4, 3, '4H'),
(24, 4, 4, '1M'),
(25, 4, 5, '5m'),
(26, 4, 6, '1m'),
(27, 4, 7, '1W'),
(28, 5, 1, '1D'),
(29, 5, 2, '1H'),
(30, 5, 3, '4H'),
(31, 5, 4, '1M'),
(32, 5, 5, '5m'),
(33, 5, 6, '1m'),
(34, 5, 7, '1W'),
(35, 10, 1, '1D'),
(36, 10, 7, '1W'),
(37, 10, 4, '1M');

INSERT IGNORE INTO `ohcl_brokers` (`id`, `name`) VALUES
(1, 'dnse'),
(2, 'binance'),
(3, 'ssi'),
(4, 'vix'),
(5, 'dragon'),
(6, 'yahoo'),
(7, 'msn'),
(10, 'gold');

INSERT IGNORE INTO `ohcl_brokers` (`id`, `name`, `alias`) VALUES
(8, 'stock', 1),
(9, 'crypto', 2);

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
(37, 2, 'future', 1),
(38, 10, 'jewelry', 1),
(39, 10, 'gold bullion', 1);

INSERT IGNORE INTO `ohcl_resolution` (`id`, `resolution`) VALUES
(1, '1D'),
(2, '1H'),
(3, '4H'),
(4, '1M'),
(5, '5m'),
(6, '1m'),
(7, '1W');


INSERT IGNORE INTO `ohcl_broker_limitation` (`id`, `broker_id`, `guest_max_history_days`, `guest_invisible`) VALUES
(1, 3, NULL, 1),
(2, 10, 7, 1);

INSERT IGNORE INTO `ohcl_symbols` (`id`, `broker_id`, `product_id`, `name`, `symbol`) VALUES
(1, 10, 39, 'Vàng miếng SJC', 'SJC'),
(2, 10, 39, 'Vàng miếng 9999', 'VM-9999'),
(3, 10, 39, 'Nhẫn tròn 9999', 'NT-9999'),
(4, 10, 38, 'Trang sức 9999', 'TS-9999'),
(5, 10, 38, 'Trang sức 999', 'TS-999'),
(6, 10, 39, 'Nhẫn tròn 999', 'NT-999'),
(7, 10, 39, 'Nhẫn tròn 99', 'NT-99'),
(8, 10, 39, 'Nhẫn tròn 98', 'NT-98')
(9, 10, 38, 'Trang sức 750', 'TS-750'),
(10, 10, 38, 'Trang sức 610', 'TS-610'),
(11, 10, 38, 'Trang sức 585', 'TS-585');
