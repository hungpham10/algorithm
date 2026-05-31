-- =========================================================================
-- 1. INSERT CHO BẢNG: ohcl_brokers
-- =========================================================================
INSERT INTO ohcl_brokers (id, name, alias) VALUES
(1, 'dnse', NULL),
(2, 'binance', NULL),
(3, 'ssi', NULL),
(4, 'vix', NULL),
(5, 'dragon', NULL),
(6, 'yahoo', NULL),
(7, 'msn', NULL),
(10, 'gold', NULL),
(8, 'stock', 1),
(9, 'crypto', 2)
ON CONFLICT (id) DO NOTHING;

SELECT setval(pg_get_serial_sequence('ohcl_brokers', 'id'), COALESCE(MAX(id), 1)) FROM ohcl_brokers;


-- =========================================================================
-- 2. INSERT CHO BẢNG: ohcl_products
-- =========================================================================
INSERT INTO ohcl_products (id, broker_id, name, enabled) VALUES
(1, 1, 'vn30', TRUE),
(2, 1, 'vn100', TRUE),
(3, 1, 'future', TRUE),
(4, 1, 'etf', TRUE),
(5, 1, 'cw', TRUE),
(6, 3, 'vn30', TRUE),
(7, 3, 'vn100', TRUE),
(8, 3, 'future', TRUE),
(9, 3, 'etf', TRUE),
(10, 3, 'cw', TRUE),
(11, 4, 'vn30', TRUE),
(12, 4, 'vn100', TRUE),
(13, 4, 'future', TRUE),
(14, 4, 'etf', TRUE),
(15, 4, 'cw', TRUE),
(16, 5, 'vn30', TRUE),
(17, 5, 'vn100', TRUE),
(18, 5, 'future', TRUE),
(19, 5, 'etf', TRUE),
(20, 5, 'cw', TRUE),
(36, 2, 'spot', TRUE),
(37, 2, 'future', TRUE),
(38, 10, 'jewelry', TRUE),
(39, 10, 'gold bullion', TRUE)
ON CONFLICT (id) DO NOTHING;

SELECT setval(pg_get_serial_sequence('ohcl_products', 'id'), COALESCE(MAX(id), 1)) FROM ohcl_products;


-- =========================================================================
-- 3. INSERT CHO BẢNG: ohcl_resolution
-- =========================================================================
INSERT INTO ohcl_resolution (id, resolution) VALUES
(1, '1D'),
(2, '1H'),
(3, '4H'),
(4, '1M'),
(5, '5m'),
(6, '1m'),
(7, '1W')
ON CONFLICT (id) DO NOTHING;

SELECT setval(pg_get_serial_sequence('ohcl_resolution', 'id'), COALESCE(MAX(id), 1)) FROM ohcl_resolution;


-- =========================================================================
-- 4. INSERT CHO BẢNG: ohcl_mapping_broker_resolution
-- =========================================================================
INSERT INTO ohcl_mapping_broker_resolution (id, broker_id, resolution_id, resolution) VALUES
(1, 1, 1, '1D'),
(2, 1, 2, '1H'),
(3, 1, 3, '4H'),
(4, 1, 4, '1M'),
(5, 1, 5, '5m'),
(6, 1, 6, '1m'),
(7, 2, 1, '1d'),
(8, 2, 2, '1h'),
(9, 2, 3, '4h'),
(10, 2, 4, '1M'),
(11, 2, 5, '5m'),
(12, 2, 6, '1m'),
(13, 2, 7, '1w'),
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
(37, 10, 4, '1M')
ON CONFLICT (id) DO NOTHING;

SELECT setval(pg_get_serial_sequence('ohcl_mapping_broker_resolution', 'id'), COALESCE(MAX(id), 1)) FROM ohcl_mapping_broker_resolution;


-- =========================================================================
-- 5. INSERT CHO BẢNG: ohcl_broker_limitation
-- =========================================================================
INSERT INTO ohcl_broker_limitation (id, broker_id, guest_max_history_days, guest_invisible) VALUES
(1, 3, NULL, TRUE),
(2, 10, 7, FALSE)
ON CONFLICT (id) DO NOTHING;

SELECT setval(pg_get_serial_sequence('ohcl_broker_limitation', 'id'), COALESCE(MAX(id), 1)) FROM ohcl_broker_limitation;


-- =========================================================================
-- 6. INSERT CHO BẢNG: ohcl_symbols
-- =========================================================================
INSERT INTO ohcl_symbols (id, broker_id, product_id, name, symbol, anchor) VALUES
(1, 10, 39, 'Vàng miếng SJC', 'SJC', 0),
(2, 10, 39, 'Vàng miếng 9999', 'VM-9999', 0),
(3, 10, 39, 'Nhẫn tròn 9999', 'NT-9999', 0),
(4, 10, 38, 'Trang sức 9999', 'TS-9999', 0),
(5, 10, 38, 'Trang sức 999', 'TS-999', 0),
(6, 10, 39, 'Nhẫn tròn 999', 'NT-999', 0),
(7, 10, 39, 'Nhẫn tròn 99', 'NT-99', 0),
(8, 10, 39, 'Nhẫn tròn 98', 'NT-98', 0),
(9, 10, 38, 'Trang sức 750', 'TS-750', 0),
(10, 10, 38, 'Trang sức 610', 'TS-610', 0),
(11, 10, 38, 'Trang sức 585', 'TS-585', 0),
(12, 10, 40, 'Bạc thỏi', 'VM-XAG', 0),
(13, 10, 41, 'Bạc trang sức', 'TS-XAG', 0),
(14, 10, 39, 'Nhẫn tròn 96', 'NT-96', 0),
(15, 10, 39, 'Vàng đúc', 'VD', 0),
(16, 10, 38, 'Trang sức 98', 'TS-98', 0),
(17, 10, 38, 'Trang sức 99', 'TS-99', 0),
(18, 10, 38, 'Tran sức 10k', 'TS-480', 0),
(19, 10, 39, 'Vàng SJC', 'SJC', 1),
(20, 10, 39, 'Vàng thế giới', 'XAU', 1),
(21, 10, 39, 'Vàng PNJ', 'PNJ', 1)
ON CONFLICT (id) DO NOTHING;

SELECT setval(pg_get_serial_sequence('ohcl_symbols', 'id'), COALESCE(MAX(id), 1)) FROM ohcl_symbols;
