import requests
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=60)
def scrape_kimkhanhviethung():
    """Scrape giá vàng từ Kim Khánh Việt Hùng và trả về danh sách dữ liệu chuẩn"""
    url = "https://kimkhanhviethung.vn/tra-cuu-gia-vang.html"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.encoding = "utf-8"

        if response.status_code != 200:
            raise RuntimeError(f"Không thể kết nối KKVH: HTTP {response.status_code}")

        soup = BeautifulSoup(response.text, "html.parser")
        table = soup.find("table", class_="table-bordered")

        if not table:
            raise ValueError(
                "Không tìm thấy bảng giá (table-bordered) trong HTML của KKVH"
            )

        prices = []
        seen_data = set()  # Dùng set để lọc trùng dựa trên nội dung thực tế

        rows = table.find_all("tr")
        for row in rows:
            tds = row.find_all("td")

            # Chỉ xử lý các dòng có đủ 3 cột (Tên, Mua, Bán)
            if len(tds) >= 3:
                name = tds[0].get_text(strip=True)

                # Làm sạch giá: xóa 'đ', dấu chấm, dấu phẩy và khoảng trắng
                raw_buy = (
                    tds[1]
                    .get_text(strip=True)
                    .replace("đ", "")
                    .replace(".", "")
                    .replace(",", "")
                    .strip()
                )
                raw_sell = (
                    tds[2]
                    .get_text(strip=True)
                    .replace("đ", "")
                    .replace(".", "")
                    .replace(",", "")
                    .strip()
                )

                # Kiểm tra tính hợp lệ của dữ liệu
                if name and raw_buy.isdigit() and raw_buy != "0":
                    # Tạo key để kiểm tra trùng lặp
                    unique_key = f"{name}-{raw_buy}-{raw_sell}"
                    if unique_key not in seen_data:
                        prices.append(
                            {"name": name, "buy": int(raw_buy), "sell": int(raw_sell)}
                        )
                        seen_data.add(unique_key)

        # Kiểm tra nếu cuối cùng không có dữ liệu nào
        if not prices:
            raise ValueError(
                "Bảng tồn tại nhưng không trích xuất được dòng giá vàng hợp lệ nào"
            )

        return prices

    except Exception as e:
        # Log lỗi chi tiết và raise để Prefect bắt được trạng thái Failed
        print(f"[{datetime.now()}] Lỗi tại Task KKVH: {e}")
        raise e
