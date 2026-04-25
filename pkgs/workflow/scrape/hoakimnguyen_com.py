import requests
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime


@task(retries=3, retry_delay_seconds=60)
def scrape_hoakimnguyen():
    """
    Cào dữ liệu giá vàng từ Hoa Kim Nguyên.
    Bắn lỗi (raise) nếu thất bại để Prefect ghi nhận trạng thái Failed và kích hoạt Retries.
    """
    url = "https://hoakimnguyen.com/"

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.encoding = "utf-8"

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(
                f"Lỗi kết nối Hoa Kim Nguyên: HTTP {response.status_code}"
            )

        soup = BeautifulSoup(response.text, "html.parser")

        # 2. Tìm bảng giá
        table = soup.find("table", class_="table-bordered")
        if not table:
            table = soup.find(
                "table", {"class": lambda x: x and "table" in str(x).lower()}
            )

        if not table:
            raise ValueError("Không tìm thấy bảng giá trên trang Hoa Kim Nguyên.")

        prices = []
        seen_data = set()
        rows = table.find_all("tr")

        for row in rows:
            if row.find("th"):  # Bỏ qua header
                continue

            tds = row.find_all("td")
            if len(tds) < 3:
                continue

            name = tds[0].get_text(strip=True)
            buy_text = tds[1].get_text(strip=True)
            sell_text = tds[2].get_text(strip=True)

            if not name or name.lower() == "loại vàng":
                continue

            # Lọc trùng lặp dữ liệu thô
            row_id = f"{name}-{buy_text}-{sell_text}"
            if row_id in seen_data:
                continue
            seen_data.add(row_id)

            # 3. Làm sạch và chuyển giá về số nguyên
            def clean_price(text):
                if not text:
                    return None
                # Chỉ lấy ký tự là số
                digit_str = "".join(filter(str.isdigit, text))
                return int(digit_str) if digit_str and digit_str != "0" else None

            try:
                buy_val = clean_price(buy_text)
                sell_val = clean_price(sell_text)
            except Exception:
                continue

            if buy_val is not None or sell_val is not None:
                prices.append({"name": name.strip(), "buy": buy_val, "sell": sell_val})

        # 4. Kiểm tra kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Truy cập được website nhưng không trích xuất được dòng dữ liệu giá vàng nào."
            )

        return prices

    except Exception as e:
        # Log chi tiết để dễ debug và raise e để Prefect bắt lỗi
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Hoa Kim Nguyên: {str(e)}")
        raise e
