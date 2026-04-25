import requests
from bs4 import BeautifulSoup
from prefect import task  # Nếu bạn dùng Prefect, nếu không hãy comment dòng này


@task(retries=3, retry_delay_seconds=60)
def scrape_ngoc_thuy():
    url = "https://tiemvangngocthuy.com/gia-vang-hom-nay/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }
    prices = []
    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")
        table = soup.find("table", class_="gold-price-table")
        if not table:
            return []

        rows = table.find("tbody").find_all("tr")
        for row in rows:
            tags = row.find_all("td")
            if len(tags) >= 3:
                name = tags[0].get_text(strip=True)
                # Lưu ý: Cột 1 là Bán, Cột 2 là Mua theo cấu trúc web này
                sell_raw = tags[1].get_text(strip=True)
                buy_raw = tags[2].get_text(strip=True)

                prices.append(
                    {
                        "source": "Ngọc Thủy",
                        "name": name,
                        "buy": int("".join(filter(str.isdigit, buy_raw))),
                        "sell": int("".join(filter(str.isdigit, sell_raw))),
                    }
                )
        return prices
    except Exception as e:
        print(f"Lỗi Ngọc Thủy: {e}")
        return []
