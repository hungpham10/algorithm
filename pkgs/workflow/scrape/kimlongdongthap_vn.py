import requests
import time
import re
from prefect import task
from datetime import datetime


def clean_gold_price(price_str):
    """Chuyển đổi '16.500.000' -> 16500000"""
    if not price_str:
        return None
    num_str = "".join(filter(str.isdigit, price_str))
    return int(num_str) if num_str else None


@task(retries=3, retry_delay_seconds=60)
def scrape_kimlong_dt():
    """Scrape giá vàng Kim Long Đồng Tháp"""
    base_url = "https://bg2.kimlongdongthap.vn/_info.aspx"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Referer": "https://bg2.kimlongdongthap.vn/BangGiaTaiQuay.aspx",
        "X-Requested-With": "XMLHttpRequest",
    }

    # Bảng ánh xạ ID sang tên sản phẩm chuẩn trên hệ thống của bạn
    gold_names = {
        1: "Vàng Nhẫn Trơn Ép Vỉ KL",
        2: "Vàng Nữ Trang 24K",
        3: "Vàng Nữ Trang 18K",
        4: "Vàng Công Ty 1",
        5: "Vàng Công Ty 2",
        8: "Vàng Nữ Trang Sáp 24K",
    }

    prices = []

    # Kim Long chạy ID từ 1 đến 8
    for i in range(1, 9):
        # Chỉ lấy các ID có trong mapping tên
        if i not in gold_names:
            continue

        params = {"ID": i, "OGP": 1, "preventCache": int(time.time() * 1000)}

        try:
            response = requests.get(
                base_url, headers=headers, params=params, timeout=15
            )
            if response.status_code == 200:
                # API này trả về text thuần, mỗi thông tin một dòng
                lines = [
                    line.strip() for line in response.text.splitlines() if line.strip()
                ]

                # Cấu trúc:: ID,: Mua,: Bán,: Xu hướng
                if len(lines) >= 3:
                    buy_val = clean_gold_price(lines)
                    sell_val = clean_gold_price(lines)

                    if buy_val or sell_val:
                        prices.append(
                            {"name": gold_names[i], "buy": buy_val, "sell": sell_val}
                        )

            # Nghỉ ngắn giữa các ID để tránh bị rate limit
            time.sleep(0.5)

        except Exception as e:
            raise e

    if not prices:
        raise (f"[{datetime.now()}] ⚠️ Không lấy được dữ liệu từ Kim Long Đồng Tháp")

    return prices
