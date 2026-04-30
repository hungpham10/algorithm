import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_ngoc_tram_q4():
    url = "https://www.tiemvangngoctramq4.com/bang-gia"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []
    try:
        response = requests.get(url, headers=headers)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")

        # 1. Tìm khung chứa bảng giá chính
        container = soup.find("div", class_="khung_giavang")
        if not container:
            return []

        # 2. Duyệt qua các dòng giá vàng (thẻ div có class là item_gv)
        rows = container.find_all("div", class_="item_gv")

        for row in rows:
            # Bỏ qua dòng tiêu đề (có class item_gv0)
            if "item_gv0" in row.get("class", []):
                continue

            # Mỗi hàng dữ liệu của Ngọc Trâm chia làm 3 cột bằng class col-4
            cols = row.find_all("div", class_="col-4")

            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)

                # Làm sạch dữ liệu: loại bỏ dấu chấm và chữ 'đ' nếu có
                buy_raw = cols[1].get_text(strip=True).replace(".", "").replace("đ", "")
                sell_raw = (
                    cols[2].get_text(strip=True).replace(".", "").replace("đ", "")
                )

                try:
                    # Ép kiểu về số nguyên
                    # Nếu giá trị là "Liên hệ" hoặc trống, khối try này sẽ nhảy sang except
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
                    # Giữ lại tên nhưng để giá là 0 hoặc bỏ qua nếu không có số
                    continue

    except Exception as e:
        # Raise lỗi để Prefect biết và thực hiện Retry
        raise e

    return prices
