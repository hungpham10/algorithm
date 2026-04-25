import requests
from bs4 import BeautifulSoup
from prefect import task  # Nếu bạn dùng Prefect, nếu không hãy comment dòng này


@task(retries=3, retry_delay_seconds=60)
def scrape_van_thong():
    url = "https://www.tiemvangvanthong.com/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }
    prices = []
    try:
        response = requests.get(url, headers=headers, timeout=15)
        soup = BeautifulSoup(response.text, "html.parser")
        table = soup.find("table", class_="table-vang")
        if not table:
            return []

        rows = table.find("tbody").find_all("tr")
        for row in rows:
            cols = row.find_all("td")
            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)
                buy_raw = cols[1].get_text(strip=True)
                sell_raw = cols[2].get_text(strip=True)

                prices.append(
                    {
                        "source": "Vân Thông",
                        "name": name,
                        "buy": int("".join(filter(str.isdigit, buy_raw))),
                        "sell": int("".join(filter(str.isdigit, sell_raw))),
                    }
                )
        return prices
    except Exception as e:
        print(f"Lỗi Vân Thông: {e}")
        return []
