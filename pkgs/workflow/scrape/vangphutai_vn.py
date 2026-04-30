import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_phu_tai():
    url = "https://vangphutai.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"

        if response.status_code != 200:
            print(f"Lỗi kết nối Phú Tài: {response.status_code}")
            return []

        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng giá dựa trên class 'table_price'
        table = soup.find("table", class_="table_price")
        if not table:
            return []

        # Lấy các dòng trong tbody (đã tự động bỏ qua thead)
        tbody = table.find("tbody")
        if not tbody:
            return []

        rows = tbody.find_all("tr")

        for row in rows:
            cols = row.find_all("td")

            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)
                buy_raw = cols[1].get_text(strip=True)
                sell_raw = cols[2].get_text(strip=True)

                # Hàm chuyển đổi chuỗi giá (ví dụ: "8.150.000") sang int
                def to_int(val):
                    digits = "".join(filter(str.isdigit, val))
                    return int(digits) if digits else 0

                buy_val = to_int(buy_raw)
                sell_val = to_int(sell_raw)

                # Chỉ thêm vào nếu có tên thương phẩm
                if name:
                    prices.append(
                        {
                            "source": "Phú Tài",
                            "name": name,
                            "buy": buy_val,
                            "sell": sell_val,
                            "unit": "VNĐ/chỉ",
                        }
                    )

        return prices
    except Exception as e:
        raise e
