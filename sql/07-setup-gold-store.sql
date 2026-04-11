-- =====================================================
-- 1. INSERT DỮ LIỆU CHO BẢNG ohcl_stores
-- =====================================================
INSERT IGNORE INTO `ohcl_stores` (`id`, `name`, `website`, `phone`) VALUES
(1, 'AJC - CÔNG TY CỔ PHẦN VÀNG BẠC ĐÁ QUÝ ASEAN', 'ajc.com.vn', '0888 865 899'),
(2, 'ANCARAT', 'ancarat.com', '19006889'),
(3, 'TẬP ĐOÀN VÀNG BẠC ĐÁ QUÝ DOJI', 'doji.vn', '1800 1168'), -- Added comma here
(4, 'CÔNG TY TNHH HẢI HỒNG JEWELRY', 'hhj.adp-p.com', '0932483248'),
(5, 'CÔNG TY TNHH HIỆU VÀNG THANH TÀU', 'hieuvangthanhtau.com.vn', '0986800979'),
(6, 'DOANH NGHIỆP TƯ NHÂN HIỆU VÀNG HOA KIM NGUYÊN', 'hoakimnguyen.com', '02363823544'),
(7, 'CÔNG TY TNHH VÀNG BẠC ĐÁ QUÝ HỒNG PHÚC GOLD', 'hongphucapp.halozend.com', '02776.268.666 - 0794.268.666'),
(8, 'CÔNG TY TNHH VÀNG BẠC ĐÁ QUÝ HUY THANH', 'huythanhjewelry.vn', '1900 633 428'),
(9, 'DOANH NGHIỆP TƯ NHÂN KIM CHÂU', 'kimchau.info', '0296 3822 344'),
(10, 'TIỆM VÀNG HỘI MỸ NGHỆ KIM HOÀN TỈNH CÀ MAU', 'www.kimhoancamau.vn', '02903 818 088 - 0918.665.255'); -- Semicolon ends the statement

-- =====================================================
-- 2. INSERT DỮ LIỆU CHO BẢNG ohcl_store_locations
-- =====================================================

-- AJC (ID từ 1)
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(1, 1, '239 Phố Vọng, Phường Tương Mai', 'Quận Hai Bà Trưng', 'Hà Nội');

-- ANCARAT (ID từ 10)
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(10, 2, '261 Nguyễn Trãi, Phường Cầu Ông Lãnh', 'Quận 1', 'TP. Hồ Chí Minh'),
(11, 2, '154 Hậu Giang, Phường Bình Tây', 'Quận 6', 'TP. Hồ Chí Minh'),
(12, 2, '382 Lê Quang Định, Phường Bình Lợi Trung', 'Quận Bình Thạnh', 'TP. Hồ Chí Minh'),
(13, 2, '105 Nguyễn Thị Thập, Phường Tân Hưng', 'Quận 7', 'TP. Hồ Chí Minh'),
(14, 2, '243 Xã Đàn, P. Đống Đa', 'Quận Đống Đa', 'Hà Nội'),
(15, 2, '47 Trần Nhân Tông, P. Hai Bà Trưng', 'Quận Hai Bà Trưng', 'Hà Nội'),
(16, 2, '24 Trần Hưng Đạo, Phường Hồng Gai', 'TP. Hạ Long', 'Quảng Ninh');

-- DOJI (ID từ 100)
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(100, 3, 'Tòa nhà DOJI Tower, Số 5 Lê Duẩn', 'Quận Ba Đình', 'Hà Nội'),
(101, 3, 'Tầng B1, Vincom Mega Mall Royal City, 72 Nguyễn Trãi', 'Quận Thanh Xuân', 'Hà Nội'),
(102, 3, 'Tầng L1, Vincom Mega Mall Smart City, Phường Tây Mỗ', 'Quận Nam Từ Liêm', 'Hà Nội'),
(103, 3, 'Tầng B1, Vincom Mega Mall Times City, 458 Minh Khai', 'Quận Hai Bà Trưng', 'Hà Nội'),
(104, 3, '243 Cầu Giấy, Phường Cầu Giấy', 'Quận Cầu Giấy', 'Hà Nội'),
(105, 3, '309 Minh Khai, Phường Vĩnh Tuy', 'Quận Hai Bà Trưng', 'Hà Nội'),
(106, 3, '11 Nguyễn Hoàng, Phường Từ Liêm', 'Quận Nam Từ Liêm', 'Hà Nội'),
(107, 3, '7 Nguyễn Hữu Thọ, Phường Hoàng Liệt', 'Quận Hoàng Mai', 'Hà Nội'),
(108, 3, '213-215 Nguyễn Văn Cừ, Phường Bồ Đề', 'Quận Long Biên', 'Hà Nội'),
(109, 3, '57 Ngô Xuân Quảng, Xã Gia Lâm', 'Huyện Gia Lâm', 'Hà Nội'),
(110, 3, '415-417 Ngọc Hồi, Xã Thanh Trì', 'Huyện Thanh Trì', 'Hà Nội'),
(111, 3, '25A Phan Đình Phùng, Phường Ba Đình', 'Quận Ba Đình', 'Hà Nội'),
(112, 3, '10A Quang Trung, Phường Hà Đông', 'Quận Hà Đông', 'Hà Nội'),
(113, 3, '22B Hai Bà Trưng, Phường Cửa Nam', 'Quận Hoàn Kiếm', 'Hà Nội'),
(114, 3, '114 Thái Hà, Phường Đống Đa', 'Quận Đống Đa', 'Hà Nội'),
(115, 3, '69 Trần Duy Hưng, Phường Yên Hòa', 'Quận Cầu Giấy', 'Hà Nội'),
(116, 3, '37B Trần Nhân Tông, Phường Hai Bà Trưng', 'Quận Hai Bà Trưng', 'Hà Nội'),
(117, 3, 'Tầng 1, Vincom Mega Mall Ocean Park, Xã Gia Lâm', 'Huyện Gia Lâm', 'Hà Nội'),
(118, 3, '476 Xã Đàn, Phường Văn Miếu – Quốc Tử Giám', 'Quận Đống Đa', 'Hà Nội'),
(119, 3, '51-53 Cao Lỗ, Xã Đông Anh', 'Huyện Đông Anh', 'Hà Nội'),
(120, 3, '146-148 Võ Văn Ngân, Phường Thủ Đức', 'TP. Thủ Đức', 'TP. Hồ Chí Minh'),
(121, 3, '231 Ba Tháng Hai, Phường Vườn Lài', 'Quận 10', 'TP. Hồ Chí Minh'),
(122, 3, '289 Quang Trung, Phường Gò Vấp', 'Quận Gò Vấp', 'TP. Hồ Chí Minh'),
(123, 3, '239 Nguyễn Thị Thập, Phường Tân Mỹ', 'Quận 7', 'TP. Hồ Chí Minh'),
(124, 3, '41A Nguyễn Ảnh Thủ, Phường Tân Thới Hiệp', 'Quận 12', 'TP. Hồ Chí Minh'),
(125, 3, '239-241 Tân Hương, Phường Phú Thọ Hòa', 'Quận Tân Phú', 'TP. Hồ Chí Minh'),
(126, 3, '66 Cách Mạng Tháng 8, Phường Xuân Hòa', 'Quận 3', 'TP. Hồ Chí Minh'),
(127, 3, '81-85 Hàm Nghi, Phường Sài Gòn', 'Quận 1', 'TP. Hồ Chí Minh'),
(128, 3, '214 Phan Đăng Lưu, Phường Đức Nhuận', 'Quận Phú Nhuận', 'TP. Hồ Chí Minh'),
(129, 3, '26/1 Nguyễn Trãi, Phường Long Xuyên', 'TP. Long Xuyên', 'An Giang'),
(130, 3, '296 Ngô Gia Tự, Phường Kinh Bắc', 'TP. Bắc Ninh', 'Bắc Ninh'),
(131, 3, '272 Trần Phú, Phường Từ Sơn', 'Thị xã Từ Sơn', 'Bắc Ninh'),
(132, 3, '29 Trần Hưng Đạo, Phường Tân Thành', 'TP. Cà Mau', 'Cà Mau'),
(133, 3, '241-243-245 Trần Hưng Đạo, Phường Quy Nhơn', 'TP. Quy Nhơn', 'Gia Lai'),
(134, 3, '55 Lý Thánh Tôn, Phường Nha Trang', 'TP. Nha Trang', 'Khánh Hòa'),
(135, 3, '89 Phan Bội Châu, Phường Xuân Hương', 'TP. Đà Lạt', 'Lâm Đồng'),
(136, 3, '188 Đường Hoàng Liên, Phường Lào Cai', 'TP. Lào Cai', 'Lào Cai'),
(137, 3, '96 Nguyễn Văn Cừ, Phường Trường Vinh', 'TP. Vinh', 'Nghệ An'),
(138, 3, 'Căn LK01, Tòa tháp Eurowindow, 02 Trần Phú', 'TP. Vinh', 'Nghệ An'),
(139, 3, '171 Biên Hòa, Phường Phủ Lý', 'TP. Phủ Lý', 'Ninh Bình'),
(140, 3, '919-921 Trần Hưng Đạo, Phường Hoa Lư', 'TP. Ninh Bình', 'Ninh Bình'),
(141, 3, '26-28 Trần Hưng Đạo, Phường Hồng Gai', 'TP. Hạ Long', 'Quảng Ninh'),
(142, 3, '214G-H Trần Phú, Tổ 6 khu 2A, Phường Cẩm Phả', 'TP. Cẩm Phả', 'Quảng Ninh'),
(143, 3, '08-10 Hùng Vương, Phường Đông Hà', 'TP. Đông Hà', 'Quảng Trị'),
(144, 3, '18 Trần Hưng Đạo, Phường Đồng Hới', 'TP. Đồng Hới', 'Quảng Trị'),
(145, 3, '31 Lương Ngọc Quyến, Phường Phan Đình Phùng', 'TP. Thái Nguyên', 'Thái Nguyên'),
(146, 3, '140-142 Lê Hoàn, Phường Hạc Thành', 'TP. Thanh Hóa', 'Thanh Hóa'),
(147, 3, '238-240 Nguyễn Tất Thành, Phường Tân Lập', 'TP. Buôn Ma Thuột', 'Đắk Lắk'),
(148, 3, '85 Phan Chu Trinh, Phường Trấn Biên', 'TP. Biên Hòa', 'Đồng Nai'),
(149, 3, '192 Đường 30 tháng 04, Phường Ninh Kiều', 'Quận Ninh Kiều', 'Cần Thơ'),
(150, 3, '75 Cầu Đất, Phường Gia Viên', 'Quận Ngô Quyền', 'Hải Phòng'),
(151, 3, '219-221 Tô Hiệu, Phường Lê Chân', 'Quận Lê Chân', 'Hải Phòng'),
(152, 3, '172 Hùng Vương, Phường Hải Châu', 'Quận Hải Châu', 'Đà Nẵng'),
(153, 3, '202-206 Lê Duẩn, Phường Thanh Khê', 'Quận Thanh Khê', 'Đà Nẵng'),
(154, 3, '105-107 Nguyễn Văn Linh, Phường Hải Châu', 'Quận Hải Châu', 'Đà Nẵng');

-- HẢI HỒNG JEWELRY
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(155, 4, '32A - 48 Trần Hưng Đạo', 'Thành phố Hải Dương', 'Hải Dương');

-- HIỆU VÀNG THANH TÀU
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(156, 5, '55 Trần Hưng Đạo, Phường Bồng Sơn', 'Thị xã Hoài Nhơn', 'Bình Định');

-- HIỆU VÀNG HOA KIM NGUYÊN
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(157, 6, '270 Ông Ích Khiêm, Phường Tân Chính', 'Quận Thanh Khê', 'Đà Nẵng');

-- HỒNG PHÚC GOLD
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(158, 7, '19-21 Lê Thị Phỉ, P. Mỹ Tho', 'Thành phố Cao Lãnh', 'Đồng Tháp');

-- HUYTHANH
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(159, 8, 'Số 10 Đội Cấn', 'Ba Đình', 'Hà Nội'),
(160, 8, 'Số 276 Nguyễn Văn Linh', 'Thanh Khê', 'Đà Nẵng'),
(161, 8, 'Số 324 Cầu Giấy', 'Cầu Giấy', 'Hà Nội'),
(162, 8, 'Số 287-289 Đường Nguyễn Văn Cừ', 'Long Biên', 'Hà Nội'),
(163, 8, 'Số 407 Bạch Mai', 'Hai Bà Trưng', 'Hà Nội'),
(164, 8, 'Số 320A Nguyễn Trãi', 'Nam Từ Liêm', 'Hà Nội'),
(165, 8, 'Số 484 - 486 Đường Cách Mạng Tháng 8', 'Quận 3', 'Hồ Chí Minh'),
(166, 8, 'Số 209 Xã Đàn', 'Đống Đa', 'Hà Nội'),
(167, 8, '221A Đường Hoàng Văn Thụ', 'Phú Nhuận', 'Hồ Chí Minh'),
(168, 8, 'Số 137 Đường Bạch Đằng', 'Bình Thạnh', 'Hồ Chí Minh'),
(169, 8, 'Số 102 Quang Trung', 'Nha Trang', 'Khánh Hòa'),
(170, 8, 'Số 3 Hậu Giang', 'Quận 6', 'Hồ Chí Minh'),
(171, 8, 'Số 1B Phố Huế', 'Hai Bà Trưng', 'Hà Nội'),
(172, 8, 'Số 136 - 138 Đường Trần Phú', 'Hải Châu', 'Đà Nẵng'),
(173, 8, '1031 Phan Văn Trị', 'Gò Vấp', 'Hồ Chí Minh'),
(174, 8, 'Số 73-75 Trần Duy Hưng', 'Cầu Giấy', 'Hà Nội'),
(175, 8, 'Số 499 Quang Trung', 'Hà Đông', 'Hà Nội'),
(176, 8, 'Số 10-12 Đường Nguyễn Trãi', 'Quận 1', 'Hồ Chí Minh'),
(177, 8, 'Số 172 Võ Văn Ngân', 'Thủ Đức', 'Hồ Chí Minh'),
(178, 8, 'PG1 - 11 Khu TM Vincom Shophouse, 209 Đường 30/4', 'Ninh Kiều', 'Cần Thơ'),
(179, 8, 'Lô 17B BT2 Bắc Linh Đàm', 'Hoàng Mai', 'Hà Nội'),
(180, 8, 'Số 24-26 Đường Phan Chu Trinh', 'Quảng Ngãi', 'Quảng Ngãi'),
(181, 8, 'Số 200-202 Lạch Tray', 'Lê Chân', 'Hải Phòng'),
(182, 8, 'Số 603 Nguyễn Thị Thập', 'Quận 7', 'Hồ Chí Minh'),
(183, 8, 'Số 63 Hồ Tùng Mậu', 'Cầu Giấy', 'Hà Nội'),
(184, 8, 'Số 133 - 135 Phố Chùa Bộc', 'Đống Đa', 'Hà Nội'),
(185, 8, 'Số 39-41 Đường Quang Trung', 'Buôn Ma Thuột', 'Đắk Lắk'),
(186, 8, 'Số 233 Đường Lê Hoàn', 'Thanh Hóa', 'Thanh Hóa'),
(187, 8, 'Số 564 Quang Trung', 'Gò Vấp', 'Hồ Chí Minh'),
(188, 8, 'K1-55 tầng 1 TTTM Tasco Mall, Số 7-9 Nguyễn Văn Linh', 'Long Biên', 'Hà Nội'),
(189, 8, 'Số 703-705 Đường Lũy Bán Bích', 'Tân Phú', 'Hồ Chí Minh'),
(190, 8, '142 Nguyễn Văn Cừ', 'Vinh', 'Nghệ An'),
(191, 8, '197 Trần Nguyên Hãn', 'Lê Chân', 'Hải Phòng'),
(192, 8, '1K76+77, Tầng 1, GO! Thăng Long, 222 Trần Duy Hưng', 'Cầu Giấy', 'Hà Nội'),
(193, 8, '689 Tôn Đức Thắng', 'Liên Chiểu', 'Đà Nẵng');

-- KIM CHÂU
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(194, 9, 'Số 16 - 18 Nguyễn Văn Linh, Tổ 5, Khóm Long Thạnh A', 'Thị xã Tân Châu', 'An Giang');

-- HỘI MỸ NGHỆ KIM HOÀN TỈNH CÀ MAU
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(195, 10, '8A Hùng Vương, P7', 'Thành phố Cà Mau', 'Cà Mau');
