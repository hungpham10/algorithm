import requests
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime

# Giả định hàm helper của bạn nằm ở đâu đó, nếu không hãy dùng logic gộp tên đơn giản
def clean_product_name(group_name, name):
    return f"{group_name} - {name}".strip()


@task(retries=3, retry_delay_seconds=60)
def scrape_kimchau_info():
    """
    Scrape giá vàng từ Kim Châu.
    Bắn lỗi (raise) nếu không tìm thấy bảng hoặc dữ liệu rỗng để Prefect Retry.
    """
    url = "https://kimchau.info/"

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
            raise RuntimeError(f"Lỗi kết nối Kim Châu: HTTP {response.status_code}")

        soup = BeautifulSoup(response.text, "html.parser")

        # 2. Tìm bảng giá (class table_bg)
        table = soup.find("table", class_="table_bg")
        if not table:
            # Ghi log file để debug sau này
            with open("kim_chau_debug.html", "w", encoding="utf-8") as f:
                f.write(response.text)
            raise ValueError("Không tìm thấy bảng giá (table_bg) trên trang Kim Châu.")

        prices = []
        rows = table.find_all("tr")

        # Bỏ qua dòng tiêu đề
        for row in rows[1:]:
            cols = row.find_all("td")
            if len(cols) < 4:
                continue

            group_name = cols[0].get_text(strip=True)

            # Lấy danh sách tên, mua, bán từ các thẻ <p>
            names = [p.get_text(strip=True) for p in cols[1].find_all("p")]
            buys = [p.get_text(strip=True) for p in cols[2].find_all("p")]
            sells = [p.get_text(strip=True) for p in cols[3].find_all("p")]

            # 3. Duyệt và làm sạch dữ liệu
            for name, buy, sell in zip(names, buys, sells):
                if not name or (not buy and not sell):
                    continue

                # Hàm clean_product_name bạn đã định nghĩa
                full_name = clean_product_name(group_name, name)

                def to_int_safe(val):
                    if not val or not val.strip():
                        return None
                    # Xóa mọi ký tự không phải số
                    digit_str = "".join(filter(str.isdigit, val))
                    return int(digit_str) if digit_str else None

                try:
                    buy_val = to_int_safe(buy)
                    sell_val = to_int_safe(sell)
                except Exception:
                    continue

                if buy_val is not None or sell_val is not None:
                    prices.append({"name": full_name, "buy": buy_val, "sell": sell_val})

        # 4. Kiểm tra kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Web Kim Châu mở được nhưng không cào được dòng giá nào. Có thể cấu trúc <p> đã đổi."
            )

        return prices

    except Exception as e:
        # Log và raise để Prefect đánh dấu Failed
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Kim Châu: {str(e)}")
        raise e
