import requests
from bs4 import BeautifulSoup
from prefect import task  # Nếu bạn dùng Prefect, nếu không hãy comment dòng này


@task(retries=3, retry_delay_seconds=60)
def scrape_xuan_tung():
    url = "https://tiemvangxuantung.com/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }
    prices = []
    try:
        response = requests.get(url, headers=headers, timeout=15)
        soup = BeautifulSoup(response.text, "html.parser")
        rows = soup.find_all("div", class_="item_gv")

        for row in rows:
            if "item_gv0" in row.get("class", []):
                continue
            cols = row.find_all("div", class_="text-center")
            if len(cols) >= 4:
                product = cols[0].get_text(strip=True)
                g_type = cols[1].get_text(strip=True)
                buy_raw = cols[2].get_text(strip=True)
                sell_raw = cols[3].get_text(strip=True)

                prices.append(
                    {
                        "source": "Xuân Tùng",
                        "name": f"{product} {g_type}",
                        "buy": int("".join(filter(str.isdigit, buy_raw))),
                        "sell": int("".join(filter(str.isdigit, sell_raw))),
                    }
                )
        return prices
    except Exception as e:
        print(f"Lỗi Xuân Tùng: {e}")
        return []
