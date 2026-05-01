import requests
import re
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime


def parse_knp_price(price_str):
    """
    Xử lý chuỗi giá: '10.200' hoặc '10.200.000' hoặc '10,200,000' -> int
    Nếu giá hiển thị đơn vị triệu (VD: 82.50) thì cần nhân thêm để ra đúng đơn vị đồng.
    """
    if not price_str or "liên hệ" in price_str.lower():
        return 0

    # Chỉ giữ lại các chữ số
    clean_str = "".join(filter(str.isdigit, price_str))

    if not clean_str:
        return 0

    val = int(clean_str)

    # Một số cửa hàng ghi giá 82.500 (hiểu là 82tr500k),
    # nếu giá trị quá nhỏ (< 1.000.000), ta nhân với 1000 hoặc tùy biến theo sàn
    if val < 200000:  # Ví dụ giá vàng nhẫn lẻ thường > 6tr
        val = val * 1000

    return val


@task(retries=3, retry_delay_seconds=60)
def scrape_kimnganphuc():
    """Scrape giá vàng Kim Ngân Phúc (Lấy tất cả các bảng giá)"""
    url = "https://kimnganphuc.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        response.raise_for_status()

        soup = BeautifulSoup(response.text, "html.parser")

        # 1. Tìm tất cả các bảng chứa giá vàng
        target_tables = []
        for t in soup.find_all("table"):
            text_content = t.get_text().upper()
            if "SẢN PHẨM" in text_content and "MUA VÀO" in text_content:
                target_tables.append(t)

        if not target_tables:
            print(
                f"[{datetime.now()}] ⚠️ Không tìm thấy bảng giá nào tại Kim Ngân Phúc"
            )
            return []

        prices = []

        # 2. Lặp qua từng bảng tìm được
        for table in target_tables:
            rows = (
                table.find("tbody").find_all("tr")
                if table.find("tbody")
                else table.find_all("tr")
            )

            for row in rows:
                cols = row.find_all("td")

                # Bỏ qua dòng header hoặc dòng trống
                if len(cols) >= 3:
                    name = cols[0].get_text(strip=True).replace("/", " ")

                    # Kiểm tra xem có phải dòng chứa dữ liệu thật không
                    if "SẢN PHẨM" in name.upper() or not name:
                        continue

                    buy_raw = cols[1].get_text(strip=True)
                    sell_raw = cols[2].get_text(strip=True)

                    buy_val = parse_knp_price(buy_raw)
                    sell_val = parse_knp_price(sell_raw)

                    if buy_val or sell_val:
                        prices.append(
                            {
                                "name": name,
                                "buy": buy_val,
                                "sell": sell_val,
                                "updated_at": datetime.now().isoformat(),  # Nên thêm timestamp để phân biệt
                            }
                        )

        if not prices:
            raise ValueError("Mở được web nhưng không trích xuất được dòng giá nào.")

        return prices

    except Exception as e:
        print(f"[{datetime.now()}] ❌ Lỗi Kim Ngân Phúc: {e}")
        raise e
