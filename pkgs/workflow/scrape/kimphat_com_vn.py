import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kimphat():
    url = "https://kimphat.evosoft.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        soup = BeautifulSoup(response.text, "html.parser")
        tables = soup.find_all("table", class_="table")

        prices = []
        for table in tables:
            # if "loại vàng" in table.get_text().lower():
            rows = table.find_all("tr")[1:]  # Bỏ qua header
            for row in rows:
                name = row.find_all("th")[0].find_all("div")[0].get_text()
                price_cols = row.find_all("td")

                if len(price_cols) >= 2:
                    buy_text = (
                        price_cols[0].find("div", class_="price").get_text(strip=True)
                    )
                    sell_text = (
                        price_cols[1].find("div", class_="price").get_text(strip=True)
                    )

                    buy = "".join(filter(str.isdigit, buy_text))
                    sell = "".join(filter(str.isdigit, sell_text))

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
        print(f"Lỗi Kim Phát: {e}")
        raise e
