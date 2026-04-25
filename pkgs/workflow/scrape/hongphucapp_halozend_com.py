import requests
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=60)
def scrape_hongphucapp_halozend():
    """
    Cào dữ liệu giá vàng từ API Hồng Phúc.
    Raise error nếu không có dữ liệu để kích hoạt cơ chế Retry của Prefect.
    """
    url = "https://hpg.hongphucdiamond.com/api/mobile-customer-app/materials/get-gold-price-short-list"

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Origin": "https://hongphucapp.halozend.com",
        "Referer": "https://hongphucapp.halozend.com/",
        "Accept": "application/json",
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối Hồng Phúc: HTTP {response.status_code}")

        res_json = response.json()

        # 2. Kiểm tra tính hợp lệ của JSON
        if not res_json or "data" not in res_json:
            raise ValueError(f"API Hồng Phúc trả về cấu trúc lạ: {res_json}")

        data = res_json.get("data", {})
        prices = []

        # Hàm helper xử lý dữ liệu
        def process_items(items):
            if not isinstance(items, list):
                return
            for item in items:
                name = item.get("name")
                if not name:
                    continue

                try:
                    # Chuyển đổi sang float và sau đó sang int để đồng bộ dữ liệu với các store khác
                    buy = int(float(item.get("price_buy", 0)))
                    sell = int(float(item.get("price", 0)))
                except (ValueError, TypeError):
                    continue

                if buy > 0 or sell > 0:
                    prices.append({"name": name.strip(), "buy": buy, "sell": sell})

        # Xử lý Vàng Đỏ và Vàng Trắng
        process_items(data.get("gold_types", []))
        process_items(data.get("white_gold_types", []))

        # 3. Kiểm tra nếu cuối cùng không có dữ liệu
        if not prices:
            raise ValueError(
                "API Hồng Phúc không trả về bất kỳ loại vàng nào có giá > 0."
            )

        return prices

    except Exception as e:
        # Log chi tiết và raise để Prefect bắt lỗi
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Hồng Phúc: {str(e)}")
        raise e
