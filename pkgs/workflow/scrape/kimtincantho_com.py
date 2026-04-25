import requests
import time
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kimtin_cantho():
    url = "https://kimtincantho.com/gia-vang-hom-nay"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.content, "html.parser")
        div_table = soup.find("div", class_="table-responsive bang-giavang")

        if not div_table:
            return []
        table = div_table.find("table")
        if not table:
            return []

        prices = []
        rows = table.find_all("tr")
        for row in rows:
            cols = row.find_all(["td", "th"])
            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)
                # Bỏ qua dòng tiêu đề
                if "loại vàng" in name.lower() or "sản phẩm" in name.lower():
                    continue

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
        print(f"Lỗi Kim Tín: {e}")
        return []
