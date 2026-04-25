import requests
from bs4 import BeautifulSoup
from prefect import task  # Nếu bạn dùng Prefect, nếu không hãy comment dòng này


@task(retries=3, retry_delay_seconds=60)
def scrape_hoang_chieu():
    url = "https://tiemvanghoangchieu.com/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []
    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng có class 'table'
        table = soup.find("table", class_="table")
        if not table:
            return []

        # Duyệt qua các dòng trong tbody
        tbody = table.find("tbody")
        rows = tbody.find_all("tr") if tbody else []

        for row in rows:
            cols = row.find_all("td", class_="item description")

            # Trường hợp dòng có đầy đủ 3 cột (Tên, Mua, Bán)
            if len(cols) == 3:
                name = cols[0].get_text(strip=True)
                buy_raw = cols[1].get_text(strip=True)
                sell_raw = cols[2].get_text(strip=True)

                # Làm sạch dữ liệu: Chỉ giữ lại số
                buy = "".join(filter(str.isdigit, buy_raw))
                sell = "".join(filter(str.isdigit, sell_raw))

                prices.append(
                    {
                        "source": "Hoàng Chiêu",
                        "name": name,
                        "buy": int(buy) if buy else 0,
                        "sell": int(sell) if sell else 0,
                        "status": "available",
                    }
                )

            # Trường hợp dòng đặc biệt (ví dụ: Vàng tiệm khác - thường dùng colspan)
            elif len(cols) == 2:
                name = cols.get_text(strip=True)
                note = cols.get_text(strip=True)

                prices.append(
                    {
                        "source": "Hoàng Chiêu",
                        "name": name,
                        "buy": 0,
                        "sell": 0,
                        "note": note,
                        "status": "info_only",
                    }
                )

        return prices

    except Exception as e:
        print(f"Lỗi khi thu thập dữ liệu Hoàng Chiêu: {e}")
        raise e
