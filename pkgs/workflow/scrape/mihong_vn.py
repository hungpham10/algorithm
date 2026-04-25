import requests
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=60)
def scrape_mihong():
    url = "https://api.mihong.vn/v1/gold-prices?market=domestic"
    headers = {
        "x-market": "mihong",
        "Origin": "https://www.mihong.vn",
        "Referer": "https://www.mihong.vn/",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    }

    try:
        prices = []
        response = requests.get(url, headers=headers, timeout=15)
        response.raise_for_status()
        for item in response.json():
            name = item.get("code", "")
            buy = int(item.get("buyingPrice", 0))
            sell = int(item.get("sellingPrice", 0))

            if name:
                prices.append({"name": name, "buy": buy, "sell": sell})
        return prices
    except Exception as e:
        print(f"Lỗi Mi Hồng: {e}")
        raise e
