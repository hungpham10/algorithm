import requests
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=30)
def scrape_hhj_adp_p_com():
    """
    Cào dữ liệu giá vàng từ HHJ ADP-P (Hieu Vang Thanh Tau).
    Bắn lỗi (raise) nếu không lấy được dữ liệu để Prefect ghi nhận trạng thái Failed.
    """
    url = "https://hieuvangthanhtau.com.vn/"

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Referer": url,
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.encoding = "utf-8"

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối HHJ ADP-P: HTTP {response.status_code}")

        soup = BeautifulSoup(response.text, "html.parser")

        # Thử tìm bảng theo class cụ thể, nếu không thấy thì tìm bảng đầu tiên
        table = soup.find("table", class_="gold-market-table") or soup.find("table")

        # 2. Kiểm tra sự tồn tại của bảng
        if not table:
            # Ghi log nội dung để debug nếu cần
            raise ValueError(
                "Không tìm thấy bảng giá vàng trên trang hieuvangthanhtau.com.vn"
            )

        prices = []
        rows = table.find_all("tr")

        for row in rows:
            if row.find("th"):  # Bỏ qua header
                continue

            cols = row.find_all("td")
            if len(cols) < 3:
                continue

            name = cols[0].get_text(strip=True)
            buy = cols[1].get_text(strip=True)
            sell = cols[2].get_text(strip=True) if len(cols) > 2 else None

            if not name or "loại vàng" in name.lower():
                continue

            # 3. Làm sạch và ép kiểu dữ liệu
            try:
                # Xóa tất cả ký tự không phải số (đ, ., ,, khoảng trắng)
                clean_buy = "".join(filter(str.isdigit, buy)) if buy else ""
                clean_sell = "".join(filter(str.isdigit, sell)) if sell else ""

                buy_val = int(clean_buy) if clean_buy else None
                sell_val = int(clean_sell) if clean_sell else None
            except Exception:
                continue

            if buy_val is not None or sell_val is not None:
                prices.append({"name": name, "buy": buy_val, "sell": sell_val})

        # 4. Kiểm tra danh sách kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Truy cập được website nhưng không trích xuất được dòng dữ liệu giá vàng nào."
            )

        return prices

    except Exception as e:
        # Log lỗi chi tiết và Re-raise để Prefect đánh dấu Task thất bại
        print(f"[{datetime.now()}] ❌ Lỗi tại Task HHJ ADP-P: {str(e)}")
        raise e
