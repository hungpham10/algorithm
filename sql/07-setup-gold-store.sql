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
(10, 'TIỆM VÀNG HỘI MỸ NGHỆ KIM HOÀN TỈNH CÀ MAU', 'www.kimhoancamau.vn', '02903 818 088 - 0918.665.255'),
(11, 'VÀNG KIM KHÁNH VIỆT HÙNG', 'kimkhanhviethung.vn', '0943.989.599'),
(12, 'CÔNG TY VÀNG BẠC ĐÁ QUÝ KIM LONG ĐỒNG THÁP', 'www.kimlongdongthap.vn', '02773.871.444 - 0919.66.00.66'),
(13, 'VÀNG KIM NGÂN PHÚC', 'kimnganphuc.vn', '1900 63 8880 - 024 66823437'),
(14, 'DNTN KINH DOANH VÀNG - CẦM ĐỒ KIM PHÁT GIA HUY', 'www.kimphat.com.vn', '0888.40.58.58'),
(15, 'CÔNG TY TNHH VÀNG BẠC - ĐÁ QUÝ KIM TÀI NGỌC', 'kimtaingocdiamond.com', '02633 864 221 - 0982 864 221'),
(16, 'DOANH NGHIỆP TƯ NHÂN KINH DOANH VÀNG KIM THÀNH.H', 'kimthanhh.com', '(028) 38753 450 - (028) 37542 450'),
(17, 'TIỆM VÀNG KIM TÍN CẦN THƠ', 'kimtincantho.com', '0939.86.89.87'),
(18, 'VÀNG MI HỒNG', 'www.mihong.vn', '+84 (28) 3841 0068 - +84 (28) 3841 0954'),
(19, 'CÔNG TY TNHH KINH DOANH VÀNG - ĐÁ QUÝ MINH VŨ', 'minhvujewelry.vn', '0965 046 046'),
(20, 'CÔNG TY TNHH VBĐQ NGỌC BÌNH', 'www.ngocbinh.com.vn', '0789 780 780 - 0788 780 780'),
(21, 'DNTN HIỆU VÀNG NGỌC THỊNH', 'ngocthinh-jewelry.vn', '0935262648'),
(22, 'TẬP ĐOÀN VÀNG BẠC ĐÁ QUÝ PHÚ QUÝ', 'phuquy.com.vn', '1800 599 995'),
(23, 'CÔNG TY VÀNG BẠC ĐÁ QUÝ SÀI GÒN - SJC', 'sjc.com.vn', '028 3929 6016'),
(24, 'CÔNG TY TNHH VÀNG BẠC ĐÁ QUÝ HOÀNG CHIÊU', 'tiemvanghoangchieu.com', '0903 104 727 - 0899 031 882'),
(25, 'TIỆM VÀNG HỒNG NGA', 'tiemvanghongnga.com', '0277 859 9999 – 0277 858 8888'),
(26, 'TIỆM VÀNG KIM HÙNG PHÁT', 'tiemvangkimhungphat.com', '02513.947963 - 0949.708708'),
(26, 'CÔNG TY TNHH DỊCH VỤ CẦM ĐỒ KIM TRỌNG NGHĨA', 'www.kimtrongnghia.com', '0833.9999.56 - 0913.1023.23 - 0938.3838.56'),
(27, 'TIỆM VÀNG MỸ NGỌC', 'tiemvangmyngoc.com', '02513 844 111 - 0766.689.689'),
(28, 'CÔNG TY TIỆM VÀNG NGỌC THỦY', 'tiemvangngocthuy.com', '093 334 56 96'),
(29, 'TIỆM VÀNG NGỌC TRÂM QUẬN 4', 'tiemvangngoctramq4.com', '090 107 89 89'),
(30, 'CÔNG TY TNHH TM VẠN THÔNG', 'www.tiemvangvanthong.com', '028 39502746'),
(40, 'CÔNG TY TNHH KINH DOANH VÀNG BẠC ĐÁ QUÍ XUÂN TÙNG', 'tiemvangxuantung.com', '032.862.1630');

-- =====================================================
-- 2. INSERT DỮ LIỆU CHO BẢNG ohcl_store_locations
-- =====================================================

-- AJC (ID từ 1)
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(1, 1, '89 Đinh Tiên Hoàng', 'Hoàn Kiếm', 'Hà Nội'),
(2, 1, '239 Phố Vọng', 'Hai Bà Trưng', 'Hà Nội'),
(3, 1, '537 Quang Trung', 'Hà Đông', 'Hà Nội'),
(4, 1, '10A Quang Trung', 'Hà Đông', 'Hà Nội'),
(5, 1, 'Tầng 1 Nhà B, Toà Tecco, C1', 'Vinh', 'Nghệ An'),
(6, 1, 'Số 1A Cao Thắng', 'Vinh', 'Nghệ An'),
(7, 1, 'Số 83 Lê Lợi', 'Thành phố Hưng Yên', 'Hưng Yên');

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

-- Cập nhật/Bổ sung danh sách chi nhánh cho Kim Khánh Việt Hùng (Store ID: 11)
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(195, 11, '24 Âu Cơ, Phường Liên Chiểu', 'Liên Chiểu', 'Đà Nẵng')
(196, 11, '159 Tôn Đức Thắng, Phường An Khê', 'Thanh Khê', 'Đà Nẵng'),
(197, 11, '354 Hùng Vương, Phường Thanh Khê', 'Thanh Khê', 'Đà Nẵng'),
(198, 11, '86 Hoàng Xuân Hãn, Phường Cẩm Lệ', 'Cẩm Lệ', 'Đà Nẵng'),
(199, 11, '320 Lý Thường Kiệt, Phường Hội An Tây', 'Hội An', 'Quảng Nam'),
(200, 11, '799A Nguyễn Lương Bằng, Phường Hải Vân', 'Liên Chiểu', 'Đà Nẵng');

-- KIM LONG ĐỒNG THÁP
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(209, 12, '6 - 8, Lê Thị Hồng Gấm, P.2', 'Thành phố Cao Lãnh', 'Đồng Tháp');

-- KIM NGÂN PHÚC
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(210, 13, 'Số 5 Nguyễn Thị Định, Phường Yên Hòa', 'Cầu Giấy', 'Hà Nội'),
(211, 13, '140 Hoàng Quốc Việt, Phường Nghĩa Đô', 'Cầu Giấy', 'Hà Nội'),
(212, 13, '247 Đường Nguyễn Văn Cừ, Phường Bồ Đề', 'Long Biên', 'Hà Nội');

-- KIM PHÁT GIA HUY
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(213, 14, '101/5A Nguyễn Văn Tạo, Ấp 1, Xã Hiệp Phước', 'Huyện Nhà Bè', 'Hồ Chí Minh');

-- KIM TÀI NGỌC
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(214, 15, '722 Trần Phú, P3', 'Thành phố Bảo Lộc', 'Lâm Đồng');

-- KIM THÀNH H
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(215, 16, '1394 Tỉnh Lộ 10, P. Tân Tạo', 'Quận Bình Tân', 'Hồ Chí Minh');

-- KIM TÍN CẦN THƠ
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(216, 17, '171 Đường 30/4, Phường Xuân Khánh', 'Quận Ninh Kiều', 'Cần Thơ');

-- MI HỒNG
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(217, 18, '306 Bùi Hữu Nghĩa, Phường Gia Định', 'Quận Bình Thạnh', 'Hồ Chí Minh');

-- MINH VŨ
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(218, 19, '179-181 Đường 30/4, P. Ninh Kiều', 'Quận Ninh Kiều', 'Cần Thơ');

-- NGỌC BÌNH
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(219, 20, '21 - 23 Nguyễn Du, Phường An Hội', 'Thành phố Vĩnh Long', 'Vĩnh Long'),
(220, 20, '123B Đại Lộ Đồng Khởi, Phường Phú Khương', 'Thành phố Vĩnh Long', 'Vĩnh Long');

-- NGỌC THỊNH
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(221, 21, '289 Trưng Nữ Vương', 'Quận Hải Châu', 'Đà Nẵng');

-- PHÚ QUÝ
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(222, 22, 'Số 51 Quang Trung, Phường Nguyễn Trãi', 'Quận Hà Đông', 'Hà Nội'),
(223, 22, 'Số 329 Đường Cầu Giấy, Phường Quan Hoa', 'Quận Cầu Giấy', 'Hà Nội'),
(224, 22, 'Số 30 Trần Nhân Tông, Phường Nguyễn Du', 'Quận Hai Bà Trưng', 'Hà Nội'),
(225, 22, 'Số 167 Trần Não, Phường An Khánh', 'Quận 2', 'Hồ Chí Minh'),
(226, 22, 'Số 193B Nam Kỳ Khởi Nghĩa, Phường Võ Thị Sáu', 'Quận 3', 'Hồ Chí Minh');

-- SJC
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(227, 23, '418 - 420 Nguyễn Thị Minh Khai, Phường Bàn Cờ', 'Quận 3', 'Hồ Chí Minh'),
(228, 23, '196 Trần Hưng Đạo, Phường Cầu Ông Lãnh', 'Quận 1', 'Hồ Chí Minh'),
(229, 23, '172 Nguyễn Văn Nghi, Phường An Nhơn', 'Gò Vấp', 'Hồ Chí Minh'),
(230, 23, '146 Bến Vân Đồn, Phường Khánh Hội', 'Quận 4', 'Hồ Chí Minh'),
(231, 23, '230 - 230A Quang Trung', 'Gò Vấp', 'Hồ Chí Minh'),
(232, 23, 'Gian L1-K6, Vincom Thảo Điền, 161 Võ Nguyên Giáp', 'Quận 2', 'Hồ Chí Minh'),
(233, 23, 'Gian L1-K6A, Vincom Lê Văn Việt, 50 Lê Văn Việt', 'Quận 9', 'Hồ Chí Minh'),
(234, 23, 'Gian T-03, Sense City, 240-242 Kha Vạn Cân', 'Thủ Đức', 'Hồ Chí Minh'),
(235, 23, 'Gian JW-2, Aeon Tân Phú, 30 Tân Thắng', 'Tân Phú', 'Hồ Chí Minh'),
(236, 23, 'Gian M10, B1, Vạn Hạnh Mall, 11 Sư Vạn Hạnh', 'Quận 10', 'Hồ Chí Minh'),
(237, 23, 'Gian 33A-33B, Lầu 2, Saigon Centre, 67 Lê Lợi', 'Quận 1', 'Hồ Chí Minh'),
(238, 23, 'Gian L1-K8, Vincom Mega Mall Grand Park', 'Quận 9', 'Hồ Chí Minh'),
(239, 23, 'Gian SBA01, Aeon Bình Tân, Số 1 Đường 17A', 'Bình Tân', 'Hồ Chí Minh'),
(240, 23, 'L2-W05, Parc Mall, 547 – 549 Tạ Quang Bửu', 'Quận 8', 'Hồ Chí Minh'),
(241, 23, 'Gian F2-K02, Hùng Vương Plaza, 126 Hồng Bàng', 'Quận 5', 'Hồ Chí Minh'),
(242, 23, 'SH8, Tòa 01,02 CCCT Trần Hưng Đạo', 'Hạ Long', 'Quảng Ninh'),
(243, 23, '89-91 Cầu Đất, Phường Gia Viên', 'Ngô Quyền', 'Hải Phòng'),
(244, 23, '27B Phan Đình Phùng, Phường Ba Đình', 'Ba Đình', 'Hà Nội'),
(245, 23, '18 Trần Nhân Tông, Phường Hai Bà Trưng', 'Hai Bà Trưng', 'Hà Nội'),
(246, 23, '50 Giang Văn Minh, Phường Ba Đình', 'Ba Đình', 'Hà Nội'),
(247, 23, '101 - 102 A49 Thái Thịnh', 'Đống Đa', 'Hà Nội'),
(248, 23, 'Vincom Plaza Hạ Long, Khu cột Đồng Hồ', 'Hạ Long', 'Quảng Ninh'),
(249, 23, 'Số 07 Hùng Vương', 'Huế', 'Thừa Thiên Huế'),
(250, 23, '185 Nguyễn Văn Linh', 'Hải Châu', 'Đà Nẵng'),
(251, 23, '193 Hùng Vương', 'Hải Châu', 'Đà Nẵng'),
(252, 23, '222 Lê Trung Đình', 'Quảng Ngãi', 'Quảng Ngãi'),
(253, 23, '13 Ngô Gia Tự', 'Nha Trang', 'Khánh Hòa'),
(254, 23, '216 Đường 30/4', 'Biên Hòa', 'Đồng Nai'),
(255, 23, 'Gian T25, Sense City, 01 Đại Lộ Hòa Bình', 'Ninh Kiều', 'Cần Thơ'),
(256, 23, '135 Đường Trần Hưng Đạo', 'Ninh Kiều', 'Cần Thơ'),
(257, 23, 'Số 205, Đường Trần Phú', 'Bạc Liêu', 'Bạc Liêu'),
(258, 23, '4A-5A Hùng Vương, Phường Tân Thành', 'Cà Mau', 'Cà Mau'),
(259, 23, 'Gian T18, Sense City, 09 Trần Hưng Đạo', 'Cà Mau', 'Cà Mau'),
(260, 23, '423 Đường Hai Tháng Tư', 'Nha Trang', 'Khánh Hòa');

-- HOÀNG CHIÊU
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(261, 24, '348-348A Tô Ngọc Vân, Phường Tam Phú', 'Thành phố Thủ Đức', 'Hồ Chí Minh');

-- HỒNG NGA
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(262, 25, 'Số 48, Đốc Binh Kiều, Phường 2', 'Thành phố Cao Lãnh', 'Đồng Tháp');

-- KIM HƯNG PHÁT
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(263, 24, '1529 đường Nguyễn Ái Quốc, phường Tân Tiến', 'Thành phố Biên Hòa', 'Đồng Nai');

-- MỸ NGỌC
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(265, 26, '506 Đường Lê Duẩn', 'Huyện Long Thành', 'Đồng Nai');

-- NGỌC THỦY
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(276, 37, '182 Đ. Kênh Tân Hóa, Phường Phú Trung', 'Quận Tân Phú', 'Hồ Chí Minh');

-- NGỌC TRÂM Q4
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(277, 38, '99-99A Tôn Thất Thuyết, Phường 15', 'Quận 4', 'Hồ Chí Minh');

-- VẠN THÔNG
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(278, 39, '808 Hưng Phú, Phường 10', 'Quận 8', 'Hồ Chí Minh');

-- XUÂN TÙNG
INSERT IGNORE INTO `ohcl_store_locations` (`id`, `store`, `address_line`, `district`, `province`) VALUES
(279, 40, '62 Nguyễn Đình Chiểu, Phường 2', 'Thành phố Cao Lãnh', 'Đồng Tháp');
