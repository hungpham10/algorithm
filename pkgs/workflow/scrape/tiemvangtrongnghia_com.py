import requests
from bs4 import BeautifulSoup
from prefect import task  # Nếu bạn dùng Prefect, nếu không hãy comment dòng này


@task(retries=3, retry_delay_seconds=60)
def scrape_kim_trong_nghia():
    url = "https://banggia.tiemvangtrongnghia.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }
    prices = []
    try:
        response = requests.get(url, headers=headers, timeout=15)
        soup = BeautifulSoup(response.text, "html.parser")
        table = soup.find("table")
        if not table:
            return []

        rows = table.find_all("tr")
        for row in rows[1:]:  # Bỏ qua header
            th_tag = row.find("th")
            cols = row.find_all("td")
            if th_tag and len(cols) >= 2:
                name = th_tag.find("div").get_text(strip=True)
                buy_div = cols[0].find("div", class_="price")
                sell_div = cols[1].find("div", class_="price")

                buy_raw = buy_div.get_text(strip=True) if buy_div else "0"
                sell_raw = sell_div.get_text(strip=True) if sell_div else "0"

                prices.append(
                    {
                        "source": "Kim Trọng Nghĩa",
                        "name": name,
                        "buy": int("".join(filter(str.isdigit, buy_raw)))
                        if buy_raw != "0"
                        else 0,
                        "sell": int("".join(filter(str.isdigit, sell_raw)))
                        if sell_raw != "0"
                        else 0,
                    }
                )
        return prices
    except Exception as e:
        print(f"Lỗi Kim Trọng Nghĩa: {e}")
        return []
