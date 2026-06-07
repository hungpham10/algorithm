from datetime import datetime
from bs4 import BeautifulSoup
from prefect import task
import requests


@task(retries=2, retry_delay_seconds=60)
def scrape_vangbaclienthanh_com():
    """Cào dữ liệu giá vàng từ Vàng Bạc Liên Thanh (https://vangbaclienthanh.com/).

    Bắn lỗi (raise) nếu không lấy được dữ liệu để Prefect ghi nhận trạng thái
    Failed.
    """
    url = "https://vangbaclienthanh.com/"

    headers = {
        "User-Agent": (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            " (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"
        ),
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Referer": url,
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.encoding = "utf-8"

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(
                f"Lỗi kết nối Vàng Bạc Liên Thanh: HTTP {response.status_code}"
            )

        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng dựa trên các style inline đặc trưng từ HTML mẫu
        # Bạn có thể điều chỉnh lại selector gọn hơn (ví dụ: soup.find("table")) nếu cấu trúc thực tế không đổi
        table = soup.find(
            "table", style=lambda s: s and "border-collapse:collapse" in s
        ) or soup.find("table")

        # 2. Kiểm tra sự tồn tại của bảng
        if not table:
            raise ValueError(f"Không tìm thấy bảng giá vàng trên trang {url}")

        prices = []
        rows = table.find_all("tr")

        for row in rows:
            # Bỏ qua dòng header chứa thẻ <th> (TÊN LOẠI | MUA VÀO | BÁN RA)
            if row.find("th"):
                continue

            cols = row.find_all("td")
            # Đảm bảo dòng có cấu trúc 3 cột dữ liệu
            if len(cols) < 3:
                continue

            name = cols[0].get_text(strip=True)
            buy = cols[1].get_text(strip=True)
            sell = cols[2].get_text(strip=True)

            if not name or "tên loại" in name.lower() or "mua vào" in name.lower():
                continue

            # 3. Làm sạch và xử lý ép kiểu dữ liệu số
            try:
                # Xóa tất cả ký tự không phải số (ví dụ: "13.750" -> "13750")
                clean_buy = "".join(filter(str.isdigit, buy)) if buy else ""
                clean_sell = "".join(filter(str.isdigit, sell)) if sell else ""

                # Ép sang int nếu có dữ liệu số, ngược lại (chữ "liên hệ") sẽ trả về None
                buy_val = int(clean_buy) if clean_buy else None
                sell_val = int(clean_sell) if clean_sell else None

                # ĐỒNG BỘ ĐƠN VỊ ĐỒNG:
                # Nếu giá trị thô nhỏ hơn 100000 (ví dụ: 13750 hoặc 150 cho bạc),
                # nhân thêm 1000 để đưa về đúng đơn vị VNĐ đầy đủ giống API bên HHJ (13750000)
                if buy_val is not None and buy_val < 100000:
                    buy_val *= 1000
                if sell_val is not None and sell_val < 100000:
                    sell_val *= 1000

            except Exception:
                # Bỏ qua dòng lỗi dữ liệu cá biệt để không chết cả pipeline
                continue

            if buy_val is not None or sell_val is not None:
                prices.append({"name": name, "buy": buy_val, "sell": sell_val})

        # 4. Kiểm tra danh sách kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Truy cập được website nhưng không trích xuất được dòng dữ liệu"
                " giá vàng nào hợp lệ."
            )

        return prices

    except Exception as e:
        # Log lỗi chi tiết và Re-raise để Prefect đánh dấu Task thất bại
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Liên Thanh: {str(e)}")
        raise e
