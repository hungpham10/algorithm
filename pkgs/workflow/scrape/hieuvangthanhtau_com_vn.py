import requests
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=60)
def scrape_hieuvangthanhtau():
    """
    Cào dữ liệu giá vàng từ Hiệu Vàng Thanh Tàu.
    Bắn lỗi (raise) nếu thất bại để Prefect ghi nhận trạng thái Failed.
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
            raise RuntimeError(
                f"Lỗi kết nối Hiệu Vàng Thanh Tàu: HTTP {response.status_code}"
            )

        soup = BeautifulSoup(response.text, "html.parser")

        # 2. Tìm bảng giá
        table = soup.find("table", class_="gold-market-table")
        if not table:
            table = soup.find(
                "table",
                class_=lambda x: x and ("gold" in x.lower() or "vàng" in x.lower()),
            )

        if not table:
            raise ValueError(
                "Không tìm thấy bảng giá vàng trên trang Hiệu Vàng Thanh Tàu."
            )

        prices = []
        rows = table.find_all("tr")

        for row in rows:
            if row.find("th"):  # Bỏ qua dòng tiêu đề
                continue

            cols = row.find_all("td")
            if len(cols) < 3:
                continue

            name = cols[0].get_text(strip=True)
            if not name or name.lower() == "loại vàng":
                continue

            # Xử lý chỉ số cột linh hoạt (thường cột 3 là icon hoặc trống, cột 4 là giá bán)
            buy_text = cols[1].get_text(strip=True)
            sell_idx = 3 if len(cols) >= 4 else 2
            sell_text = cols[sell_idx].get_text(strip=True)

            # 3. Làm sạch và chuyển giá về số nguyên
            def clean_price(text):
                if not text:
                    return None
                # Chỉ lấy các ký tự là số
                digit_str = "".join(filter(str.isdigit, text))
                return int(digit_str) if digit_str and digit_str != "0" else None

            try:
                buy_val = clean_price(buy_text)
                sell_val = clean_price(sell_text)
            except:
                continue

            if buy_val is not None or sell_val is not None:
                prices.append({"name": name.strip(), "buy": buy_val, "sell": sell_val})

        # 4. Kiểm tra kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Website truy cập được nhưng không trích xuất được dòng giá vàng nào."
            )

        return prices

    except Exception as e:
        # Log lỗi chi tiết và ném lỗi lên Prefect
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Thanh Tàu: {str(e)}")
        raise e
