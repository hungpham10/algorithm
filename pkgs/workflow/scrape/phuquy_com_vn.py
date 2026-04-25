import requests
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_phuquy():
    # URL API mới từ yêu cầu của bạn
    url = "https://be.phuquy.com.vn/jewelry/product-payment-service/api/sync-price-history/get-sync-table-history"

    headers = {
        "Origin": "https://phuquy.com.vn",
        "Referer": "https://phuquy.com.vn/",
        "Accept": "application/json, text/plain, */*",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    }

    try:
        # Sử dụng timeout để tránh treo task
        response = requests.get(url, headers=headers, timeout=20)
        response.raise_for_status()
        result = response.json()

        # Kiểm tra mã lỗi từ API Phú Quý
        if result.get("errorCode") == "0":
            items = result.get("data", [])
            prices = []

            for item in items:
                # Trích xuất thông tin dựa trên cấu trúc JSON mới
                name = item.get("productTypeName", "").strip()
                buy_price = item.get("priceIn")
                sell_price = item.get("priceOut")

                # Chỉ thêm vào danh sách nếu có giá trị hợp lệ
                if name and buy_price is not None:
                    prices.append(
                        {
                            "name": name,
                            "buy": int(buy_price),
                            "sell": int(sell_price) if sell_price else 0,
                            "last_update": item.get("lastUpdate"),
                            "unit": item.get("unitOfMeasure") or "Vnđ/lượng",
                        }
                    )
            return prices

        print(f"API trả về lỗi: {result.get('message')}")
        return []

    except Exception as e:
        # Log lỗi để Prefect ghi nhận
        print(f"Lỗi khi cào dữ liệu Phú Quý: {e}")
        return []


# Cách sử dụng kết quả (Ví dụ)
# for p in scrape_phuquy():
#     print(f"{p['name']}: Mua {p['buy']:,} - Bán {p['sell']:,} ({p['unit']})")
