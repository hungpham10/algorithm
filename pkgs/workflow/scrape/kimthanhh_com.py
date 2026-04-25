import requests
import time
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kimthanhh():
    url = "https://kimthanhh.com/gia-vang"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        soup = BeautifulSoup(response.text, "html.parser")
        table_goldprice = soup.find("div", class_="table-goldprice")

        if not table_goldprice:
            return []

        results = []
        brands = table_goldprice.find_all("div", class_="column-brandgold")
        details = table_goldprice.find_all("div", class_="column-typegold")

        for brand, detail in zip(brands, details):
            name = brand.get_text(strip=True)
            box = detail.find("div", class_="box-typegold")
            if box:
                # Lấy số, bỏ chữ "Liên hệ"
                buy_raw = box.find("div", class_="col-buy").get_text(strip=True)
                sell_raw = box.find("div", class_="col-sell").get_text(strip=True)

                buy = "".join(filter(str.isdigit, buy_raw))
                sell = "".join(filter(str.isdigit, sell_raw))

                if buy or sell:
                    results.append(
                        {
                            "name": name,
                            "buy": int(buy) if buy else 0,
                            "sell": int(sell) if sell else 0,
                        }
                    )
        return results
    except Exception as e:
        print(f"Lỗi Kim Thành H: {e}")
        return []
