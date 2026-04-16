-- AJC (ID từ 1)

INSERT IGNORE INTO `ohcl_mapping_product_in_store_to_symbol`
	(`id`, `symbol`, `store`, `product_name`)
VALUES
	(1, 3, 1, 'NL 99.90'),
	(2, 5, 1, 'Trang sức 99.9'),
	(3, 4, 1, 'Trang sức 99.99');

INSERT IGNORE INTO `ohcl_mapping_product_in_store_to_symbol`
	(`id`, `symbol`, `store`, `product_name`, `scope`, `location`)
VALUES
	(4, 1, 1, 'Miếng SJC Hà Nội', 1, 1),
	(5, 1, 1, 'Miếng SJC Nghệ An', 1, 5),
	(6, 1, 1, 'Miếng SJC Thái Bình', 1, 7),
	(7, 3, 1, 'N.Tròn, 3A, Đ.Vàng H.Nội', 1, 1),
	(8, 3, 1, 'N.Tròn, 3A, Đ.Vàng N.An', 1, 5),
	(9, 3, 1, 'N.Tròn, 3A, Đ.Vàng T.Bình', 1, 7),
	(10, 3, 1, 'NL 99.99, Nhẫn Tròn Thái Bình', 1, 7);

-- ANCARAT (ID từ 10)
INSERT IGNORE INTO `ohcl_mapping_product_in_store_to_symbol`
	(`id`, `symbol`, `store`, `product_name`)
VALUES
	(11, 2, 2, 'Vàng Kim Ấn Trần Triều 9999 (1 chỉ)'),
	(12, 2, 2, 'Vàng Nam Kim Thành 9999 (1 chỉ)'),
	(13, 2, 2, 'Vàng Bắc Sư Tử 9999 (1 chỉ)'),
	(14, 2, 2, 'Set Vàng Tứ Linh Hội Tụ 9999 (0.1 chỉ x 4)'),
	(15, 2, 2, 'Vàng Kim Long Quảng Tiến 9999 (0.1 chỉ)'),
	(16, 2, 2, 'Vàng Kim Lân Thịnh Vượng 9999 (0.1 chỉ)'),
	(17, 2, 2, 'Vàng Kim Quy Tụ Tài 9999 (0.1 chỉ)'),
	(18, 2, 2, 'Vàng Kim Phụng Cát Tường 9999 (0.1 chỉ)'),
	(19, 2, 2, 'Vàng Kim Thần Tài 9999 (1 chỉ)'),
	(20, 2, 2, 'Vàng Kim Thần Tài 9999 (0.5 chỉ)'),
	(21, 2, 2, 'Set combo Bắc Sư Tử 1 chỉ Vàng 9999 và 1 lượng bạc 999 (500 bản)'),
	(22, 3, 2, 'Vàng Nhẫn Tích Tài 9999 (1 chỉ)'),
	(23, 3, 2, 'Nhẫn Vàng ACR 9999 (1 chỉ)'),
	(24, 6, 2, 'Nhẫn Vàng ACR 999 (1 chỉ)'),
	(25, 7, 2, 'Nhẫn Vàng ACR 99 (1 chỉ)'),
	(26, 8, 2, 'Nhẫn Vàng ACR 98 (1 chỉ)'),
	(27, 4, 2, 'Trang sức Vàng 24K 9999 (1 chỉ)'),
	(28, 9, 2, 'Trang sức Vàng 18K 750 (1 chỉ)'),
	(29, 10, 2, 'Trang sức Vàng 610 (1 chỉ)'),
	(30, 11, 2, 'Trang sức vàng 14K 585 (1 chỉ)');

INSERT INTO `ohcl_mapping_product_in_store_to_symbol` (`id`, `symbol`, `store`, `product_name`, `scope`, `location`) VALUES
	(31, 13, 4, 'Bạc Trang Sức', 0, NULL),
	(32, 12, 4, 'Bạc Kim Phúc Lộc', 0, NULL);

INSERT INTO `ohcl_mapping_product_in_store_to_symbol` (`id`, `symbol`, `store`, `product_name`, `scope`, `location`) VALUES
	(33, 6, 5, 'Vàng 99.9 (lần 3)', 0, NULL),
	(34, 7, 5, 'Vàng 950 (lần 3)', 0, NULL),
	(35, 10, 5, 'VÀNG 610 (lần 1)', 0, NULL),
	(36, 10, 5, 'VÀNG 530 (lần 1)', 0, NULL),
	(37, 11, 5, 'VÀNG 416 (lần 1)', 0, NULL),
	(38, 13, 5, 'BẠC (lần 1)', 0, NULL);

INSERT INTO `ohcl_mapping_product_in_store_to_symbol` (`id`, `symbol`, `store`, `product_name`, `scope`, `location`) VALUES
	(39, 2, 6, '9999 vĩ', 0, NULL),
	(40, 3, 6, 'Vàng nhẫn khâu 9999', 0, NULL),
	(41, 8, 6, 'Vàng nhẫn khâu 98', 0, NULL),
	(42, 9, 6, 'Vàng nhẫn khâu 96', 0, NULL),
	(43, 14, 6, 'Nữ trang 980', 0, NULL),
	(44, 15, 6, 'Vàng công ty', 0, NULL),
	(45, 15, 6, 'Vàng đúc', 0, NULL);

INSERT INTO `ohcl_mapping_product_in_store_to_symbol` (`id`, `symbol`, `store`, `product_name`, `scope`, `location`) VALUES
	(46, 3, 7, 'Nhẫn Trơn 99.99 Ép Vỉ Hồng Phúc', 0, NULL),
	(47, 4, 7, '99.99%', 0, NULL),
	(48, 5, 7, '99.9%', 0, NULL),
	(49, 17, 7, '99%', 0, NULL),
	(50, 16, 7, '98%', 0, NULL),
	(51, 9, 7, '75%', 0, NULL),
	(52, 10, 7, '68%', 0, NULL),
	(53, 11, 7, '61%', 0, NULL),
	(54, 11, 7, 'VT 61%', 0, NULL);

INSERT INTO `ohcl_mapping_product_in_store_to_symbol` (`id`, `symbol`, `store`, `product_name`, `scope`, `location`) VALUES
	(55, 18, 8, '10K', 0, NULL),
	(56, 11, 8, '14K', 0, NULL),
	(57, 9, 8, '18K', 0, NULL),
	(58, 8, 8, '22K', 0, NULL),
	(59, 2, 8, '24K', 0, NULL),
	(60, 2, 8, '24KTT', 0, NULL);
