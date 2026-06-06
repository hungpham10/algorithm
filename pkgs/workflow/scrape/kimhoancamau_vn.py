import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kimhoancamau():
    url = "https://kimhoancamau.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng theo id bạn cung cấp
        table = soup.find("table", id="table_price_gold_change")

        if not table:
            return []

        prices = []
        rows = (
            table.find("tbody").find_all("tr")
            if table.find("tbody")
            else table.find_all("tr")
        )

        for row in rows:
            cols = row.find_all("td")

            # Bảng này có 4 cột: Loại, Chất lượng %, Mua vào, Bán ra
            if len(cols) == 4:
                type_gold = cols[0].get_text(strip=True)
                quality = cols[1].get_text(strip=True)

                # Bỏ qua dòng tiêu đề của bảng
                if type_gold == "Loại" or quality == "Chất lượng %":
                    continue

                # Định danh tên kết hợp từ Loại và Chất lượng (Ví dụ: "Vàng 24K (9999)")
                name = f"{type_gold} {quality}"

                # Chỉ giữ lại các chữ số cho giá mua và giá bán
                buy = "".join(filter(str.isdigit, cols[2].get_text()))
                sell = "".join(filter(str.isdigit, cols[3].get_text()))

                if buy or sell:
                    prices.append(
                        {
                            "name": name,
                            "buy": int(buy) if buy else 0,
                            "sell": int(sell) if sell else 0,
                        }
                    )
        return prices

    except Exception as e:
        print(f"Lỗi Kim Hoàn Cà Mau: {e}")
        raise e
