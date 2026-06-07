from datetime import datetime
from bs4 import BeautifulSoup
import requests
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kimtin_com_vn():
    """Cào dữ liệu giá vàng từ Tập đoàn Kim Tín (https://kimtin.com.vn/).

    Bắn lỗi (raise) nếu không lấy được dữ liệu để Prefect ghi nhận trạng thái
    Failed.
    """
    url = "https://kimtin.com.vn/"

    headers = {
        "User-Agent": (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            " (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"
        ),
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Referer": url,
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.encoding = "utf-8"

        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối Kim Tín: HTTP {response.status_code}")

        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng nằm trong class wrapper đặc trưng của Kim Tín
        table_wrapper = soup.find("div", class_="table-wrapper")
        table = table_wrapper.find("table") if table_wrapper else soup.find("table")

        if not table:
            raise ValueError(f"Không tìm thấy bảng giá vàng trên trang {url}")

        prices = []
        # Tìm tất cả các dòng trong tbody để tránh dính header trong thead
        tbody = table.find("tbody")
        rows = tbody.find_all("tr") if tbody else table.find_all("tr")

        for row in rows:
            if row.find("th"):  # Bỏ qua dòng tiêu đề nếu có
                continue

            cols = row.find_all("td")
            total_cols = len(cols)

            # Xử lý lệch cột do thuộc tính rowspan của cột đầu tiên
            if total_cols == 5:
                # Dòng đầu tiên của nhóm rowspan (Thương phẩm | Loại vàng | Hàm lượng | Mua | Bán)
                raw_name_cell = cols[1]
                content_cell = cols[2]
                buy_cell = cols[3]
                sell_cell = cols[4]
            elif total_cols == 4:
                # Dòng tiếp theo bị gộp ô (Loại vàng | Hàm lượng | Mua | Bán)
                raw_name_cell = cols[0]
                content_cell = cols[1]
                buy_cell = cols[2]
                sell_cell = cols[3]
            else:
                continue

            # Lấy div đầu tiên trong ô Loại vàng để làm tên gốc (ví dụ: "NHẪN TRÒN TRƠN")
            first_div = raw_name_cell.find("div")
            base_name = (
                first_div.get_text(strip=True)
                if first_div
                else raw_name_cell.get_text(strip=True)
            )

            # Lấy hàm lượng (ví dụ: "999.9 (24K)") để ghép vào tên cho tường minh, tránh trùng lặp
            content_text = (
                content_cell.get_text(" ", strip=True) if content_cell else ""
            )
            name = f"{base_name} {content_text}".strip()

            buy = buy_cell.get_text(strip=True)
            sell = sell_cell.get_text(strip=True)

            # 3. Làm sạch và xử lý ép kiểu dữ liệu số
            try:
                clean_buy = "".join(filter(str.isdigit, buy)) if buy else ""
                clean_sell = "".join(filter(str.isdigit, sell)) if sell else ""

                buy_val = int(clean_buy) if clean_buy else None
                sell_val = int(clean_sell) if clean_sell else None

                # Chuẩn hóa về đơn vị VNĐ đầy đủ (ví dụ: 14150 -> 14150000)
                if buy_val is not None and buy_val < 100000:
                    buy_val *= 1000
                if sell_val is not None and sell_val < 100000:
                    sell_val *= 1000
            except Exception:
                continue

            if buy_val is not None or sell_val is not None:
                prices.append({"name": name, "buy": buy_val, "sell": sell_val})

        if not prices:
            raise ValueError(
                "Truy cập được website Kim Tín nhưng không trích xuất được dòng dữ liệu nào."
            )

        return prices

    except Exception as e:
        print(f"[{datetime.now()}] ❌ Lỗi tại Task Kim Tín: {str(e)}")
        raise e
