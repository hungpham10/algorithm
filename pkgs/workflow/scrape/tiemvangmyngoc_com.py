import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_my_ngoc():
    url = "https://tiemvangmyngoc.com/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []
    try:
        response = requests.get(url, headers=headers)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")

        # 1. Tìm bảng có class="bang-gia"
        table = soup.find("table", class_="bang-gia")

        if not table:
            return []

        # 2. Lấy tất cả các dòng trong tbody (bỏ qua header nếu có)
        # Mỹ Ngọc thường để dữ liệu trực tiếp trong các thẻ <tr>
        rows = table.find_all("tr")

        for row in rows:
            cols = row.find_all("td")

            # Kiểm tra dòng có đủ 3 cột: Loại vàng - Mua vào - Bán ra
            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)

                # Tránh lấy dòng tiêu đề (ví dụ dòng có chữ "Loại vàng")
                if "Loại" in name or "Vàng" in name:
                    continue

                # Lấy nội dung text và làm sạch dấu chấm
                buy_raw = cols[1].get_text(strip=True).replace(".", "")
                sell_raw = cols[2].get_text(strip=True).replace(".", "")

                try:
                    # Ép kiểu sang int để đồng nhất dữ liệu
                    buy_val = int(buy_raw)
                    sell_val = int(sell_raw)

                    prices.append(
                        {
                            "name": name,
                            "buy": buy_val,
                            "sell": sell_val,
                        }
                    )
                except (ValueError, TypeError):
                    # Bỏ qua các dòng không phải số (ví dụ dòng trống hoặc text lạ)
                    continue

    except Exception as e:
        # Trong Prefect, raise lỗi để hệ thống ghi nhận task thất bại và thực hiện retry
        raise e

    return prices
