import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_kim_hung_phat():
    url = "https://tiemvangkimhungphat.com/bang-gia-vang"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []
    try:
        response = requests.get(url, headers=headers)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng dựa trên style hoặc cấu trúc tbody
        # Vì bảng này không có class riêng, ta tìm bảng có chiều rộng 400px như
        # trong mẫu
        table = soup.find_all("table")[3]
        rows = table.find("tbody").find_all("tr")

        for row in rows:
            cols = row.find_all("td")
            if len(cols) >= 3:
                name = cols[0].get_text(strip=True)
                buy_raw = cols[1].get("x:num")
                sell_raw = cols[2].get("x:num")

                try:
                    buy_val = int(str(buy_raw).replace(".", ""))
                    sell_val = int(str(sell_raw).replace(".", ""))

                    prices.append(
                        {
                            "name": name,
                            "buy": buy_val,
                            "sell": sell_val,
                        }
                    )
                except (ValueError, TypeError):
                    continue
    except Exception as e:
        raise e

    return prices
