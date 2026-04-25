import requests
from prefect import task
from datetime import datetime

# Đảm bảo bạn đã định nghĩa URL này hoặc truyền từ CONFIG
SJC_API_URL = (
    "https://sjc.com.vn/gia-vang-trong-nuoc"  # Thay bằng URL API thực tế của bạn
)


@task(retries=3, retry_delay_seconds=60)
def scrape_sjc():
    """
    Lấy dữ liệu từ API SJC.
    Bắn lỗi (raise) để kích hoạt cơ chế Retry và Dashboard Failed nếu API lỗi.
    """
    headers = {
        "Accept": "*/*",
        "X-Requested-With": "XMLHttpRequest",
        "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
        "Origin": "https://sjc.com.vn",
        "Referer": "https://sjc.com.vn/",
    }

    try:
        # 1. Kiểm tra HTTP Status
        response = requests.post(SJC_API_URL, headers=headers, data={}, timeout=30)

        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối SJC: HTTP {response.status_code}")

        # 2. Kiểm tra JSON hợp lệ
        result = response.json()
        if not result or "data" not in result:
            raise ValueError(f"API SJC trả về cấu trúc không mong muốn: {result}")

        raw_data = result.get("data", [])
        latest_date = result.get("latestDate", "N/A")

        print(f"[{datetime.now()}] SJC cập nhật lúc: {latest_date}")

        prices = []
        for item in raw_data:
            # Lọc: Chỉ lấy các loại vàng tại Hồ Chí Minh để tránh trùng lặp tên
            if item.get("BranchName") != "Hồ Chí Minh":
                continue

            name = item.get("TypeName", "").strip()

            # SJC trả về BuyValue/SellValue là số đã được tính toán chuẩn (thường là đơn vị VNĐ/lượng)
            # Ví dụ: 89000000
            try:
                buy_val = int(item.get("BuyValue", 0))
                sell_val = int(item.get("SellValue", 0))
            except (ValueError, TypeError):
                continue

            if name and (buy_val > 0 or sell_val > 0):
                prices.append({"name": name, "buy": buy_val, "sell": sell_val})

        # 3. Kiểm tra nếu danh sách rỗng
        if not prices:
            raise ValueError(
                "Kết nối được API SJC nhưng không tìm thấy dữ liệu vàng Hồ Chí Minh."
            )

        return prices

    except Exception as e:
        # Re-raise lỗi để Prefect ghi nhận trạng thái Failed
        print(f"[{datetime.now()}] ❌ Lỗi tại Task SJC: {str(e)}")
        raise e
