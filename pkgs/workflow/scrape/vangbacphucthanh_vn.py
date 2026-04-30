import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_phuc_thanh():
    url = "https://vangbacphucthanh.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    gold_prices = []

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"

        if response.status_code != 200:
            print(f"Lỗi kết nối Phúc Thành: {response.status_code}")
            return []

        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng chứa giá vàng dựa trên nội dung đặc trưng
        table = None
        for t in soup.find_all("table"):
            if "MUA VÀO" in t.get_text():
                table = t
                break

        if not table:
            return []

        rows = table.find_all("tr")
        for row in rows:
            cols = row.find_all("td")

            # Bảng của Phúc Thành thường có 3 cột: Loại vàng | Bán ra | Mua vào
            if len(cols) == 3:
                name = cols[0].get_text(strip=True)
                sell_raw = cols[1].get_text(strip=True)
                buy_raw = cols[2].get_text(strip=True)

                # Loại bỏ tiêu đề bảng và các dòng trống
                if (
                    any(x in name.upper() for x in ["LOẠI", "MẶT HÀNG", "GIÁ"])
                    or not buy_raw
                ):
                    continue

                try:
                    # Chuyển đổi giá về dạng số để dễ quản lý (Xử lý chuỗi "8.150" -> 8150)
                    buy_val = int("".join(filter(str.isdigit, buy_raw)))
                    sell_val = int("".join(filter(str.isdigit, sell_raw)))

                    gold_prices.append(
                        {
                            "source": "Phúc Thành",
                            "name": name,
                            "buy": buy_val,
                            "sell": sell_val,
                            "unit": "1.000 VNĐ/chỉ",
                        }
                    )
                except ValueError:
                    continue

        return gold_prices
    except Exception as e:
        raise e
