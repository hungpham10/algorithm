import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=2, retry_delay_seconds=60)
def scrape_ajc():
    """Scrape giá vàng từ AJC"""
    url = "https://www.ajc.com.vn/others/OthersHome/priceGold"

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)
        response.encoding = "utf-8"

        if response.status_code != 200:
            print(f"[{datetime.now()}] Lỗi kết nối AJC: {response.status_code}")
            return []

        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng (thử nhiều cách vì trang hay thay đổi)
        table = soup.find("table", class_="table-report")
        if not table:
            table = soup.find("table", {"class": lambda x: x and "table" in x.lower()})

        if not table:
            # Lưu debug
            with open("ajc_debug.html", "w", encoding="utf-8") as f:
                f.write(response.text)
            return []

        prices = []
        rows = (
            table.find("tbody").find_all("tr")
            if table.find("tbody")
            else table.find_all("tr")
        )

        for row in rows:
            cols = row.find_all("td")
            if len(cols) < 3:
                continue

            name = cols[0].get_text(strip=True)
            if not name or "loại vàng" in name.lower():
                continue

            # Lấy giá mua và bán, làm sạch
            try:
                buy_text = (
                    cols[1]
                    .get_text(strip=True)
                    .replace(".", "")
                    .replace(",", "")
                    .replace(" ", "")
                )
                sell_text = (
                    cols[2]
                    .get_text(strip=True)
                    .replace(".", "")
                    .replace(",", "")
                    .replace(" ", "")
                    if len(cols) > 2
                    else ""
                )

                buy = int(buy_text) if buy_text.isdigit() else 0
                sell = int(sell_text) if sell_text.isdigit() else 0
            except:
                continue

            if buy is not None or sell is not None:
                prices.append({"name": name, "buy": buy, "sell": sell})

        if len(prices) == 0:
            raise Exception(
                "Dữ liệu từ ajc.com.vn trả về rỗng, làm ơn kiểm tra hệ thống"
            )
        return prices

    except Exception as e:
        print(f"[{datetime.now()}] Lỗi scrape AJC: {e}")
        raise e
