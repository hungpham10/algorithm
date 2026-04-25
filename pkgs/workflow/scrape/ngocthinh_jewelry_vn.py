import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_ngocthinh():
    url = "https://ngocthinh-jewelry.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    }

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"
        soup = BeautifulSoup(response.text, "html.parser")
        content_area = soup.find("div", class_="contenttogoldflex")

        if not content_area:
            return []

        prices = []
        rows = content_area.find_all("div", recursive=False)

        for row in rows:
            name_tag = row.find("div", class_="headerindex1")
            buy_tag = row.find("div", class_="headerindex2")
            sell_tag = row.find("div", class_="headerindex3")

            if name_tag and buy_tag and sell_tag:
                name = name_tag.get_text(strip=True).replace("\xa0", " ")
                buy = "".join(filter(str.isdigit, buy_tag.get_text()))
                sell = "".join(filter(str.isdigit, sell_tag.get_text()))

                if buy or sell:
                    prices.append(
                        {
                            "name": name,
                            "buy": int(buy) if buy else 0,
                            "sell": int(sell) if sell else 0,
                        }
                    )
        return prices
    except Exception as e:
        print(f"Lỗi Ngọc Thịnh: {e}")
        return []
