import requests
import time
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_baotinkk():
    url = "https://baotinkk.com/pages/gia-vang"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.content, "html.parser")

        # Tìm bảng theo class đặc trưng của baotinkk từ HTML bạn cung cấp
        table = soup.find(
            "table",
            class_="gold-table__table-avunzsxhgs0p4c3hangoldpricestablelivelkch4n",
        )

        # Dự phòng nếu class động thay đổi, tìm theo table thông thường
        if not table:
            table = soup.find("table")

        if not table:
            return []

        prices = []
        rows = table.find_all("tr")
        for row in rows:
            # Lấy tất cả các ô td hoặc th trong dòng
            cols = row.find_all(["td", "th"])
            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)

                # Bỏ qua dòng tiêu đề của bảng
                if "loại vàng" in name.lower() or "sản phẩm" in name.lower():
                    continue

                # Lọc lấy số (loại bỏ dấu chấm, chữ /chỉ, /gram)
                buy_text = cols[1].get_text()
                sell_text = cols[2].get_text()

                buy = "".join(filter(str.isdigit, buy_text))
                sell = "".join(filter(str.isdigit, sell_text))

                # Chỉ thêm vào danh sách nếu có tên sản phẩm hợp lệ
                if name:
                    prices.append(
                        {
                            "name": name,
                            "buy": int(buy) if buy else 0,
                            "sell": int(sell) if sell else 0,
                        }
                    )

        return prices

    except Exception as e:
        print(f"Lỗi Bảo Tín Kim Kiệt: {e}")
        return []
