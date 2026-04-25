import requests
from bs4 import BeautifulSoup
from prefect import task  # Nếu bạn dùng Prefect, nếu không hãy comment dòng này


@task(retries=3, retry_delay_seconds=60)
def scrape_hong_nga():
    url = "http://tiemvanghongnga.com/gia-vang"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }
    prices = []
    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")
        container = soup.find("div", class_="khunggiavang")
        if not container:
            return []

        items = container.find_all("div", class_="gold_item")
        for item in items:
            tags = item.find("div")
            if tags:
                name = tags.find("div", class_="name").get_text(strip=True)
                # Tìm tất cả các cột giá mua/bán
                price_cols = tags.find_all("div", class_="col_buy_price")
                if len(price_cols) >= 2:
                    buy_raw = price_cols[0].get_text(strip=True)
                    sell_raw = price_cols[1].get_text(strip=True)

                    buy = "".join(filter(str.isdigit, buy_raw))
                    sell = "".join(filter(str.isdigit, sell_raw))

                    prices.append(
                        {
                            "source": "Hồng Nga",
                            "name": name,
                            "buy": int(buy) if buy else 0,
                            "sell": int(sell) if sell else 0,
                        }
                    )
        return prices
    except Exception as e:
        print(f"Lỗi Hồng Nga: {e}")
        return []
