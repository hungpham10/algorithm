import requests
from bs4 import BeautifulSoup
from prefect import task


@task(retries=3, retry_delay_seconds=60)
def scrape_chien_minh():
    url = "https://www.vangchienminh.vn/"
    headers = {
        "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }

    prices = []

    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.encoding = "utf-8"

        if response.status_code != 200:
            print(f"Lỗi kết nối Chiến Minh: {response.status_code}")
            return []

        soup = BeautifulSoup(response.text, "html.parser")

        # Tìm bảng chứa text "LOẠI VÀNG"
        table = None
        for t in soup.find_all("table"):
            if "LOẠI VÀNG" in t.get_text():
                table = t
                break

        if not table:
            return []

        rows = table.find_all("tr")

        # Duyệt từ dòng thứ 1 để bỏ qua tiêu đề
        for row in rows[1:]:
            cols = row.find_all(["th", "td"])

            if len(cols) >= 4:
                # Xử lý text thô
                name = cols[0].get_text(strip=True).replace("\xa0", " ")
                content = cols[1].get_text(strip=True)
                buy_raw = cols[2].get_text(strip=True)
                sell_raw = cols[3].get_text(strip=True)

                # Hàm helper để parse số (ví dụ: "8.150.000" -> 8150000)
                def parse_price(val):
                    digits = "".join(filter(str.isdigit, val))
                    return int(digits) if digits else 0

                buy_val = parse_price(buy_raw) * 1000
                sell_val = parse_price(sell_raw) * 1000

                # Chỉ thêm vào danh sách nếu có giá mua hoặc bán hợp lệ
                if buy_val > 0 or sell_val > 0:
                    prices.append(
                        {
                            "source": "Chiến Minh",
                            "name": name,
                            "buy": buy_val,
                            "sell": sell_val,
                            "unit": "VNĐ/lượng",  # Chiến Minh thường tính theo lượng hoặc chỉ tùy bảng, bạn kiểm tra lại đơn vị nhé
                        }
                    )

        return prices

    except Exception as e:
        raise e
