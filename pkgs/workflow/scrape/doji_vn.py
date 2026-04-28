import requests
import json
import re
from bs4 import BeautifulSoup
from prefect import task
from datetime import datetime


def clean_product_name(name):
    # Fix lỗi hiển thị: loại bỏ khoảng trắng đặc biệt và các ký tự lạ
    name = name.replace("\xa0", " ")
    # Xử lý các trường hợp tên bị dính ký tự thừa và khoảng trắng dư
    name = re.sub(r"\s+", " ", name).strip()
    return name


@task(retries=3, retry_delay_seconds=60)
def scrape_doji():
    """Scrape giá vàng Doji - Fix triệt để lỗi BÃ¡n Láº» (Encoding)"""
    url = "https://giavang.doji.vn/?q=doji/get/json/gia_vang_quoc_te"

    headers = {
        "Accept": "application/json, text/javascript, */*",
        "X-Requested-With": "XMLHttpRequest",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Referer": "https://giavang.doji.vn/",
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.raise_for_status()

        # --- PHẦN QUAN TRỌNG: XỬ LÝ ENCODING ---
        # 1. Lấy nội dung thô và giải mã bằng utf-8-sig (loại bỏ BOM)
        content_text = response.content.decode("utf-8-sig")

        # 2. Fix lỗi escape JSON (các dấu \ không hợp lệ)
        fixed_json = re.sub(r'(?<!\\)\\(?!"|\\|/|b|f|n|r|t|u)', r"\\\\", content_text)

        data = json.loads(fixed_json, strict=False)
        html_raw = data.get("main_price", "")

        if not html_raw:
            return []

        # 3. Nếu html_raw vẫn chứa các ký tự hex như \x3c, giải mã nó
        if "\\x" in html_raw:
            # unicode_escape sẽ biến \x3c thành < và xử lý các ký tự đặc biệt
            html_raw = html_raw.encode("utf-8").decode("unicode_escape")

        # 4. Đôi khi dữ liệu bị ép kiểu latin-1, ta ép ngược về utf-8
        # Đây là bước "chốt hạ" để trị lỗi BÃ¡n Láº»
        try:
            html_raw = html_raw.encode("latin-1").decode("utf-8")
        except:
            pass  # Nếu không phải lỗi latin-1 thì bỏ qua

        soup = BeautifulSoup(html_raw, "html.parser")
        tbody = soup.find("tbody")
        if not tbody:
            return []

        prices = []
        rows = tbody.find_all("tr")
        for row in rows:
            name_tag = row.find("span", class_="title")
            if not name_tag:
                continue

            # Lấy tên và làm sạch
            product_name = clean_product_name(name_tag.get_text())

            # Lấy giá
            buy_tag = row.find("td", class_="goldprice-td-0")
            sell_tag = row.find("td", class_="goldprice-td-1")

            def parse_price(tag):
                if not tag:
                    return None
                text = "".join(filter(str.isdigit, tag.get_text()))
                return int(text) if text else None

            buy_val = parse_price(buy_tag) * 1000
            sell_val = parse_price(sell_tag) * 1000

            if buy_val or sell_val:
                prices.append({"name": product_name, "buy": buy_val, "sell": sell_val})

        return prices

    except Exception as e:
        print(f"[{datetime.now()}] ❌ Lỗi scrape Doji: {str(e)}")
        raise e
