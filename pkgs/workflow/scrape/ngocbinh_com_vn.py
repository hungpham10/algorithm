import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_ngocbinh():
    url = "https://www.ngocbinh.com.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")
        items = soup.find_all("div", class_="giavangindex")

        prices = []
        for item in items:
            # Bỏ qua dòng tiêu đề
            if "giavangindex0" in item.get("class", []):
                continue

            divs = item.find_all("div")
            if len(divs) >= 3:
                name = divs[0].get_text(strip=True)
                buy = "".join(filter(str.isdigit, divs[1].get_text()))
                sell = "".join(filter(str.isdigit, divs[2].get_text()))

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
        print(f"Lỗi Ngọc Bình: {e}")
        return []
