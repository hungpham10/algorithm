import requests
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_phuquy():
    url = "https://be.phuquy.com.vn/jewelry/product-payment-service/api/products/get-price"
    headers = {
        "Origin": "https://phuquy.com.vn",
        "Referer": "https://phuquy.com.vn/",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.raise_for_status()
        result = response.json()

        if result.get("errorCode") == "0":
            items = result.get("data", [])
            prices = []
            for item in items:
                # API Phú Quý trả về giá theo đơn vị 'Chỉ'
                # Nếu hệ thống của bạn dùng đơn vị 'Lượng', hãy nhân với 10
                prices.append(
                    {
                        "name": item.get("name", "").strip(),
                        "buy": int(item.get("buyprice", 0)),
                        "sell": int(item.get("sellprice", 0)),
                    }
                )
            return prices
        return []
    except Exception as e:
        print(f"Lỗi Phú Quý: {e}")
        return []
