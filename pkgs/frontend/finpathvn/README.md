# 📊 FinpathVN - Nền Tảng Theo Dõi & So Sánh Tài Sản Tài Chính

**FinpathVN** là hệ thống dữ liệu tài chính đa tài sản, cung cấp thông tin thời gian thực về Vàng, Ngoại tệ (USD/VND), Dầu thô, Chứng khoán Việt Nam và Chứng khoán Mỹ. Nền tảng không chỉ dừng lại ở việc hiển thị giá mà còn cung cấp bộ công cụ phân tích, giúp người dùng so sánh hiệu suất sinh lời giữa các kênh đầu tư khác nhau.

## 🏗️ Kiến Trúc Hệ Thống (Advanced Hybrid Stack)

Để xử lý khối lượng dữ liệu khổng lồ từ nhiều thị trường khác nhau, dự án áp dụng:

- **Frontend:** [Astro](https://astro.build/) (SSR) tối ưu hóa tốc độ tải trang cho các dashboard phức tạp.
- **Reactivity:** [Svelte](https://svelte.dev/) đảm nhận các bộ công cụ tính toán lợi nhuận và cập nhật giá Realtime.
- **Data Architecture:**
    - **GraphQL API:** Gateway hợp nhất dữ liệu từ nhiều Module (Gold, Forex, Stock, Oil). Lấy dữ liệu lịch sử phục vụ vẽ biểu đồ so sánh.
    - **REST API:** Truy xuất dữ liệu giá biến động nhanh phục vụ cập nhật tức thời (Live-price).

## 🚀 Danh Mục Tài Sản Hỗ Trợ

- **Vàng:** SJC, PNJ, Nhẫn trơn, Vàng thế giới.
- **Ngoại tệ:** Tỷ giá trung tâm, tỷ giá các ngân hàng thương mại (USD, EUR, JPY...).
- **Năng lượng:** Giá dầu thô (WTI, Brent).
- **Chứng khoán:** Chỉ số VN-Index, HNX-Index và các chỉ số chính của Chứng khoán Mỹ (S&P 500, Nasdaq, Dow Jones).

## 🧮 Bộ Công Cụ Phân Tích & Tính Toán

FinpathVN cung cấp các công cụ độc quyền giúp nhà đầu tư cá nhân tối ưu hóa danh mục:

- **Công cụ tính lợi nhuận:** Tính toán lãi lỗ dựa trên giá mua/bán thực tế của từng loại tài sản.
- **Biểu đồ so sánh lợi nhuận:** Vẽ biểu đồ trực quan so sánh tỷ suất sinh lời của Vàng vs. Chứng khoán vs. Gửi tiết kiệm ngân hàng trong cùng một khoảng thời gian.
- **Giả lập đầu tư:** Công cụ dự báo lợi nhuận dựa trên lãi suất kép và biến động lịch sử của tài sản.

## 📱 Trải Nghiệm Người Dùng & Quảng Cáo

- **UI/UX:** Thiết kế tinh gọn, cỡ chữ lớn, dễ thao tác cho người cao tuổi nhưng vẫn đầy đủ tính năng cho nhà đầu tư chuyên nghiệp.
- **Smart Ads Logic:**
    - **Desktop:** Sidebar thông minh với các Banner dính (Sticky) tăng tỷ lệ chuyển đổi.
    - **Mobile:** Hệ thống quảng cáo tối giản, tập trung vào **Sticky Footer Banner** để không làm gián đoạn trải nghiệm đọc dữ liệu.

## 🛠️ Lệnh Chạy Dự Án

| Lệnh | Tác dụng |
| :--- | :--- |
| `npm install` | Cài đặt toàn bộ dependencies |
| `npm run dev` | Khởi chạy môi trường phát triển |
| `npm run build` | Build dự án cho môi trường Production |

---
**FinpathVN** - *Đồng hành cùng hành trình tự do tài chính của người Việt.*
