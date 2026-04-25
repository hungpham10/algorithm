import requests
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=60)
def scrape_huythanhjewelry():
    """
    Cào dữ liệu giá vàng từ API Huy Thanh Jewelry.
    Sử dụng raise để Prefect ghi nhận trạng thái Failed nếu không có dữ liệu.
    """
    url = "https://huythanhjewelry.vn/api/gold"

    headers = {
        "Accept": "application/json, text/javascript, */*",
        "X-Requested-With": "XMLHttpRequest",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Referer": "https://huythanhjewelry.vn/gia-vang-hom-nay",
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối Huy Thanh: HTTP {response.status_code}")

        items = response.json()

        # 2. Kiểm tra dữ liệu JSON có phải là list không
        if not isinstance(items, list):
            raise ValueError(
                f"API Huy Thanh trả về định dạng không phải list: {type(items)}"
            )

        if not items:
            raise ValueError("API Huy Thanh trả về danh sách rỗng.")

        prices = []
        for item in items:
            loai = item.get("loaivang", "")
            tuoi = item.get("tuoi", "")
            mua = item.get("giamua")
            ban = item.get("giaban")

            if not loai:
                continue

            # 3. Xử lý chuyển đổi giá trị số an toàn
            def to_int_price(val):
                if val is None or val == "":
                    return None
                try:
                    # Xử lý nếu giá là string có dấu phẩy hoặc float
                    clean_val = float(str(val).replace(",", ""))
                    return int(clean_val) if clean_val > 0 else None
                except (ValueError, TypeError):
                    return None

            buy_val = to_int_price(mua)
            sell_val = to_int_price(ban)

            if buy_val is not None or sell_val is not None:
                # Kết hợp loại vàng và tuổi vàng để ra tên sản phẩm đầy đủ
                full_name = f"{loai.strip()} {tuoi.strip()}".strip()
                prices.append(
                    {
                        "name": full_name,
                        "buy": buy_val,
                        "sell": sell_val,
                    }
                )

        # 4. Kiểm tra kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Không trích xuất được bất kỳ dữ liệu giá vàng hợp lệ nào từ Huy Thanh."
            )

        return prices

    except Exception as e:
        # Log lỗi và raise để Prefect xử lý (Retry hoặc đánh dấu Failed)
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Huy Thanh: {str(e)}")
        raise e
