import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kimtaingoc():
    url = "https://kimtaingocdiamond.com/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")
        table = soup.find("table", class_="jd-border-0")

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
            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)
                # Loại bỏ " VNĐ" hoặc dấu chấm/phẩy
                buy = "".join(filter(str.isdigit, cols[1].get_text()))
                sell = "".join(filter(str.isdigit, cols[2].get_text()))

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
        print(f"Lỗi Kim Tài Ngọc: {e}")
        raise e
