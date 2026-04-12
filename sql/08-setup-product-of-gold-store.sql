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
	(21, 2, 2, 'Set combo Bắc Sư Tử 1 chỉ Vàng 9999 và 1 lượn'),
	(22, 3, 2, 'Vàng Nhẫn Tích Tài 9999 (1 chỉ)'),
	(23, 3, 2, 'Nhẫn Vàng ACR 9999 (1 chỉ)'),
	(24, 6, 2, 'Nhẫn Vàng ACR 999 (1 chỉ)'),
	(25, 7, 2, 'Nhẫn Vàng ACR 99 (1 chỉ)'),
	(26, 8, 2, 'Nhẫn Vàng ACR 98 (1 chỉ)'),
	(27, 4, 2, 'Trang sức Vàng 24K 9999 (1 chỉ)'),
	(28, 9, 2, 'Trang sức Vàng 18K 750 (1 chỉ)'),
	(29, 10, 2, 'Trang sức Vàng 610 (1 chỉ)'),
	(30, 11, 2, 'Trang sức vàng 14K 585 (1 chỉ)');
