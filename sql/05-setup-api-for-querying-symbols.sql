INSERT IGNORE INTO `sys_api_map` (`id`, `tenant_id`, `mode`, `name`, `url`, `parser`) VALUES
(1, 1, 4, 'get-all-symbols-in-vn30', 'https://bgapidatafeed.vps.com.vn/getlistckindex/VN30', '[\"Iter\"]'),
(2, 1, 4, 'get-all-symbols-in-vn100', 'https://bgapidatafeed.vps.com.vn/getlistckindex/VN100', '[\"Iter\"]'),
(3, 1, 4, 'get-all-symbols-in-penny', 'https://bgapidatafeed.vps.com.vn/getlistckindex/VNSML', '[\"Iter\"]'),
(4, 1, 4, 'get-all-symbols-in-midcap', 'https://bgapidatafeed.vps.com.vn/getlistckindex/VNMID', '[\"Iter\"]'),
(5, 1, 4, 'get-all-symbols-in-etf', 'https://bgapidatafeed.vps.com.vn/getlistckindex/hsx_e', '[\"Iter\"]'),
(6, 1, 4, 'get-exchange-rate-from-vcb', 'https://www.vietcombank.com.vn/api/exchangerates?date={}', '[{\"Match\": \"Data\"}, \"Iter\", {\"Select\": [\"currencyCode\", \"sell\"]}]');
