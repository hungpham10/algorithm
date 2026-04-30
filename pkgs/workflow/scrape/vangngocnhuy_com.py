import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_ngoc_nhu_y():
    url = "https://vangngocnhuy.com/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"

        if response.status_code != 200:
            print(f"Lỗi kết nối Ngọc Như Ý: {response.status_code}")
            return []

        soup = BeautifulSoup(response.text, "html.parser")
        container = soup.find("div", class_="khung_giavang1")

        if not container:
            return []

        rows = container.find_all("div", class_="item_gv")

        for row in rows:
            # Bỏ qua dòng tiêu đề
            if "item_gv0" in row.get("class", []):
                continue

            cols = row.find_all("div", recursive=False)

            if len(cols) >= 3:
                # 1. Lấy tên và làm sạch
                name = cols[0].get_text(strip=True)

                # 2. Lấy giá thô
                buy_raw = cols[1].get_text(strip=True)
                sell_raw = cols[2].get_text(strip=True)

                # 3. Hàm xử lý số (Chuyển "8.150.000" hoặc "Liên hệ" về số nguyên)
                def clean_price(val):
                    digits = "".join(filter(str.isdigit, val))
                    return int(digits) if digits else 0

                buy_val = clean_price(buy_raw)
                sell_val = clean_price(sell_raw)

                # Chỉ lưu nếu có tên sản phẩm
                if name:
                    prices.append(
                        {
                            "source": "Ngọc Như Ý",
                            "name": name,
                            "buy": buy_val,
                            "sell": sell_val,
                            "unit": "VNĐ/chỉ",  # Đơn vị mặc định thường dùng
                        }
                    )

        return prices
    except Exception as e:
        raise e
