import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_btmc():
    url = "https://btmc.vn/gia-vang-theo-ngay.html"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }
    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.content, "html.parser")

        # Tìm bảng giá chính
        tables = soup.find_all("table", class_="bd_price_home")
        table = tables[0] if tables else soup.find("table")

        if not table:
            return []

        prices = []
        rows = table.find_all("tr")

        for row in rows:
            # Tìm tên loại vàng (các dòng chứa "VÀNG", "NHẪN", "TRANG SỨC", "QUÀ", "SJC"...)
            name = None
            for col in row.find_all(["td", "th"]):
                text = col.get_text(strip=True)
                if text and (
                    "VÀNG" in text.upper()
                    or "NHẪN" in text.upper()
                    or "TRANG SỨC" in text.upper()
                    or "QUÀ" in text.upper()
                    or "SJC" in text.upper()
                ):
                    name = text
                    break

            if not name:
                continue

            # Tìm giá mua/bán qua div có id bắt đầu bằng mua_/ban_
            mua_div = row.find("div", id=lambda x: x and x.startswith("mua_"))
            ban_div = row.find("div", id=lambda x: x and x.startswith("ban_"))

            buy = 0
            sell = 0

            if mua_div:
                buy_text = mua_div.get_text(strip=True)
                buy = (
                    int("".join(filter(str.isdigit, buy_text)))
                    if any(c.isdigit() for c in buy_text)
                    else 0
                )

            if ban_div:
                sell_text = ban_div.get_text(strip=True)
                sell = (
                    int("".join(filter(str.isdigit, sell_text)))
                    if any(c.isdigit() for c in sell_text)
                    else 0
                )

            # Bỏ qua header và dòng rỗng
            if (
                name
                and ("Thương phẩm" not in name and "Loại vàng" not in name)
                and (buy > 0 or sell > 0)
            ):
                prices.append(
                    {
                        "name": name,
                        "buy": buy,
                        "sell": sell,
                    }
                )

        # Loại bỏ trùng lặp (do HTML có lặp bảng)
        unique_prices = []
        seen = set()
        for p in prices:
            key = p["name"].strip()
            if key and key not in seen:
                seen.add(key)
                unique_prices.append(p)

        return unique_prices

    except Exception as e:
        print(f"Lỗi BTMC: {e}")
        return []
