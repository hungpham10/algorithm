from datetime import datetime
import requests
from prefect import task

# Định nghĩa bảng map mã tỷ giá sang tên loại vàng
# Bạn có thể bổ sung thêm các mã khác (như 201, 184, 185...) vào đây dựa trên thực tế website hiển thị
GOLD_MAP = {
    "82": "Nhẫn Tròn Hải Hồng 9999",
    "186": "Nhẫn Tròn Hải Hồng 999",
    "201": "Trang sức 995",
    "184": "Trang sức 999",
    "185": "Trang sức 9999",
    "211": "Trang sức 980",
    "106": "Bạc Trang Sức",
    "235": "Bạc Kim Phúc Lộc tham khảo",
    "195": "NL 9999 tham khảo",
}


@task(retries=3, retry_delay_seconds=30)
def scrape_hhj_adp_p_com():
    """Cào dữ liệu giá vàng trực tiếp từ API endpoint của HHJ và map sang tên

    tường minh. Bắn lỗi (raise) nếu không lấy được hoặc không parse được dữ liệu
    để Prefect ghi nhận trạng thái Failed.
    """
    url = "http://hhj.adp-p.com/Public/get_TyGia?p="

    headers = {
        "User-Agent": (
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15"
            " (KHTML, like Gecko) Version/18.6 Safari/605.1.15"
        ),
        "Accept": "text/plain, */*; q=0.01",
        "Referer": "http://hhj.adp-p.com/tygia/banggia",
        "X-Requested-With": "XMLHttpRequest",
    }

    try:
        response = requests.get(url, headers=headers, timeout=20)

        # 1. Kiểm tra HTTP Status
        if response.status_code != 200:
            raise RuntimeError(f"Lỗi kết nối API HHJ: HTTP {response.status_code}")

        # 2. Parse JSON response
        try:
            raw_data = response.json()
        except Exception as e:
            raise ValueError(
                f"Không thể parse JSON từ endpoint. Response text sơ bộ:"
                f" {response.text[:100]}... Lỗi: {str(e)}"
            )

        if not isinstance(raw_data, list):
            raise ValueError("Định dạng API thay đổi, không trả về một list.")

        prices = []

        # 3. Duyệt qua mảng dữ liệu và làm sạch dữ liệu số
        for item in raw_data:
            code = str(item.get("matygia", "")).strip()

            # Lấy tên từ bảng map, nếu không tìm thấy mã trong danh sách thì dùng chính mã tỷ giá làm tên
            name = GOLD_MAP.get(code, code)

            buy = item.get("giamua")
            sell = item.get("giaban")

            if not code:
                continue

            try:
                # Xử lý xóa dấu phẩy phân tách phần ngàn (ví dụ: "13,700" -> "13700")
                clean_buy = buy.replace(",", "").strip() if buy else ""
                clean_sell = sell.replace(",", "").strip() if sell else ""

                buy_val = int(clean_buy) if clean_buy.isdigit() else None
                sell_val = int(clean_sell) if clean_sell.isdigit() else None
            except Exception:
                continue

            if buy_val is not None or sell_val is not None:
                prices.append(
                    {"name": name, "buy": buy_val * 1000, "sell": sell_val * 1000}
                )

        # 4. Kiểm tra danh sách kết quả cuối cùng
        if not prices:
            raise ValueError(
                "Gọi API thành công nhưng không bóc tách được bản ghi hợp lệ nào."
            )

        return prices

    except Exception as e:
        # Log lỗi chi tiết và Re-raise để Prefect đánh dấu Task thất bại
        print(f"[{datetime.now()}] ❌ Lỗi tại Task API HHJ: {str(e)}")
        raise e
