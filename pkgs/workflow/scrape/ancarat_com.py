import requests
from prefect import task
from datetime import datetime


@task(retries=2, retry_delay_seconds=60)
def scrape_ancarat():
    """Lấy dữ liệu giá vàng từ Ancarat API"""
    url = "https://ancarat.com/api/price-data"
    params = {"type": "gold"}
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Referer": "https://ancarat.com/gia-vang",
        "Accept": "application/json",
    }

    try:
        response = requests.get(url, params=params, headers=headers, timeout=20)

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối Ancarat: HTTP {response.status_code}")

        data = response.json()  # Giả sử trả về mảng 2 chiều

        # 2. Kiểm tra format dữ liệu JSON
        if not data or not isinstance(data, list):
            raise ValueError("Dữ liệu từ API Ancarat không đúng định dạng hoặc rỗng")

        prices = []
        update_time = None

        for i, row in enumerate(data):
            if not row or len(row) < 3:
                continue

            name = str(row[0]).strip() if row else ""

            # Xóa prefix '- ' nếu có ở đầu name
            if name.startswith("- "):
                name = name[2:].strip()

            buy = row[1]
            sell = row[2]

            # Lấy thời gian cập nhật nếu có
            if i == 1 and isinstance(sell, str) and ("/" in sell or ":" in sell):
                update_time = sell
                continue

            # Bỏ qua tiêu đề hoặc dòng rỗng
            if not name or "Mã Tham Chiếu" in name or "GTGT" in name.upper():
                continue

            # Chuyển giá về số
            try:
                buy_val = (
                    int(str(buy).replace(".", "").replace(",", ""))
                    if buy not in [None, "", " "]
                    else None
                )
                sell_val = (
                    int(str(sell).replace(".", "").replace(",", ""))
                    if sell not in [None, "", " "]
                    else None
                )
            except Exception as e:
                buy_val = None
                sell_val = None

            if buy_val is not None or sell_val is not None:
                prices.append({"name": name, "buy": buy_val, "sell": sell_val})

        # 3. Kiểm tra xem cuối cùng có lấy được giá nào không
        if not prices:
            raise ValueError(
                "API trả về dữ liệu nhưng không trích xuất được giá vàng hợp lệ nào"
            )

        if update_time:
            print(f"[{datetime.now()}] Cập nhật Ancarat: {update_time}")

        return prices

    except Exception as e:
        # Re-raise lỗi để Prefect bắt được trạng thái Failed
        # In log ra để dễ debug trên console/cloud
        print(f"[{datetime.now()}] Lỗi scrape Ancarat: {e}")
        raise e
