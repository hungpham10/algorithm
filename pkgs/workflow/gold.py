import os
import requests
from bs4 import BeautifulSoup
from prefect import flow, get_run_logger, task
from prefect.blocks.system import Secret

from scrape import (
    scrape_ajc,
    scrape_ancarat,
    scrape_doji,
    scrape_hhj_adp_p_com,
    scrape_hieuvangthanhtau,
    scrape_hoakimnguyen,
    scrape_hongphucapp_halozend,
    scrape_huythanhjewelry,
    scrape_kimchau_info,
    scrape_kimkhanhviethung,
    scrape_sjc,
    scrape_kimlong_dt,
    scrape_kimnganphuc,
    scrape_kimphat,
    scrape_kimtaingoc,
    scrape_kimthanhh,
    scrape_kimtin_cantho,
    scrape_mihong,
    scrape_minhvu,
    scrape_ngocbinh,
    scrape_ngocthinh,
    scrape_phuquy,
    scrape_hoang_chieu,
    scrape_hong_nga,
    scrape_ngoc_thuy,
    scrape_ngoc_tram_q4,
    scrape_my_ngoc,
    scrape_kim_trong_nghia,
    scrape_van_thong,
    scrape_xuan_tung,
    scrape_kim_hung_phat,
)

# --- CONFIG LOADER ---
def load_config():
    """
    Nạp cấu hình an toàn.
    Các thông tin nhạy cảm được lấy từ Prefect Secret Blocks.
    Các thông tin định danh/URL được lấy từ Env hoặc gán mặc định.
    """
    try:
        client_secret = Secret.load("findaily-upsert-client-secret").get()
        client_id = Secret.load("findaily-upsert-client-id").get()
        audience = Secret.load("findaily-upsert-audience").get()
        api_base = Secret.load("findaily-upsert-api-base").get()
    except Exception as e:
        raise e

    return {
        "AUTH0_URL": "https://universal-lazycat-auth.us.auth0.com/oauth/token",
        "CLIENT_ID": client_id,
        "CLIENT_SECRET": client_secret,
        "AUDIENCE": audience,
        "API_BASE": api_base,
    }


# Khởi tạo config một lần
CONFIG = load_config()

# --- TASKS ---
@task(retries=3, retry_delay_seconds=10)
def get_token():
    """Lấy token từ Auth0 để xác thực API."""
    if not CONFIG["CLIENT_SECRET"]:
        raise ValueError("CLIENT_SECRET is missing! Check Prefect Blocks or Env.")

    resp = requests.post(
        CONFIG["AUTH0_URL"],
        json={
            "client_id": CONFIG["CLIENT_ID"],
            "client_secret": CONFIG["CLIENT_SECRET"],
            "audience": CONFIG["AUDIENCE"],
            "grant_type": "client_credentials",
        },
        timeout=15,
    )
    resp.raise_for_status()
    return resp.json()["access_token"]


@task(retries=0, retry_delay_seconds=5, tags=["price-api-limit"])
def upload_store_prices(token, store_name, prices):
    """Task tải dữ liệu lên server và thu thập lỗi chi tiết."""
    logger = get_run_logger()

    if not token:
        raise Exception("Seem to be token is expired or failed to setup")

    if not prices:
        raise Exception("No collected price")

    success_count = 0
    error_details = []

    for item in prices:
        try:
            url = f"{CONFIG['API_BASE']}/stores/{store_name}/products/{item['name']}/price"
            headers = {
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            }
            payload = {"buy": float(item["buy"]), "sell": float(item["sell"])}

            resp = requests.post(url, json=payload, headers=headers, timeout=10)

            if resp.status_code == 200:
                success_count += 1
            else:
                server_error = resp.json() if resp.content else "No response body"
                error_msg = f"Product '{item['name']}': {server_error}"
                error_details.append(error_msg)
        except Exception as e:
            error_details.append(f"Product '{item['name']}': System Error {str(e)}")

    if error_details:
        total_errors = len(error_details)
        summary_error = (
            f"Thất bại khi publish {total_errors} sản phẩm tại {store_name}."
        )

        # Log một bản tóm tắt cuối cùng trước khi raise
        logger.error(f"--- TỔNG KẾT LỖI [{store_name}] ---")
        for err in error_details:
            logger.error(err)

        raise Exception(summary_error)

    return {
        "store": store_name,
        "total": len(prices),
        "success": success_count,
        "failed": len(error_details),
        "errors": error_details,
    }


# --- FLOW ---
@flow(name="Gold-Price-Sync-Optimized")
def gold_sync_flow():
    logger = get_run_logger()

    # 1. Lấy Token xác thực
    token = get_token()

    # 2. Registry danh sách các nguồn
    scrape_registry = [
        (scrape_ajc, "AJC - CÔNG TY CỔ PHẦN VÀNG BẠC ĐÁ QUÝ ASEAN"),
        (scrape_ancarat, "ANCARAT"),
        (scrape_doji, "TẬP ĐOÀN VÀNG BẠC ĐÁ QUÝ DOJI"),
        (scrape_hhj_adp_p_com, "DOANH NGHIỆP TƯ NHÂN KIM CHÂU"),
        (scrape_hieuvangthanhtau, "CÔNG TY TNHH HIỆU VÀNG THANH TÀU"),
        (scrape_hoakimnguyen, "DOANH NGHIỆP TƯ NHÂN HIỆU VÀNG HOA KIM NGUYÊN"),
        (scrape_hongphucapp_halozend, "CÔNG TY TNHH VÀNG BẠC ĐÁ QUÝ HỒNG PHÚC GOLD"),
        (scrape_huythanhjewelry, "CÔNG TY TNHH VÀNG BẠC ĐÁ QUÝ HUY THANH"),
        (scrape_kimchau_info, "DOANH NGHIỆP TƯ NHÂN KIM CHÂU"),
        (scrape_kimkhanhviethung, "VÀNG KIM KHÁNH VIỆT HÙNG"),
        (scrape_kimlong_dt, "CÔNG TY VÀNG BẠC ĐÁ QUÝ KIM LONG ĐỒNG THÁP"),
        (scrape_kimnganphuc, "VÀNG KIM NGÂN PHÚC"),
        (scrape_kimphat, "DNTN KINH DOANH VÀNG - CẦM ĐỒ KIM PHÁT GIA HUY"),
        (scrape_kimtaingoc, "CÔNG TY TNHH VÀNG BẠC - ĐÁ QUÝ KIM TÀI NGỌC"),
        (scrape_kimthanhh, "DOANH NGHIỆP TƯ NHÂN KINH DOANH VÀNG KIM THÀNH.H"),
        (scrape_kimtin_cantho, "TIỆM VÀNG KIM TÍN CẦN THƠ"),
        (scrape_mihong, "VÀNG MI HỒNG"),
        (scrape_minhvu, "CÔNG TY TNHH KINH DOANH VÀNG - ĐÁ QUÝ MINH VŨ"),
        (scrape_ngocbinh, "CÔNG TY TNHH VBĐQ NGỌC BÌNH"),
        (scrape_ngocthinh, "DNTN HIỆU VÀNG NGỌC THỊNH"),
        (scrape_phuquy, "TẬP ĐOÀN VÀNG BẠC ĐÁ QUÝ PHÚ QUÝ"),
        # (scrape_sjc, "CÔNG TY VÀNG BẠC ĐÁ QUÝ SÀI GÒN - SJC"),
        (scrape_hoang_chieu, "CÔNG TY TNHH VÀNG BẠC ĐÁ QUÝ HOÀNG CHIÊU"),
        (scrape_hong_nga, "TIỆM VÀNG HỒNG NGA"),
        (scrape_kim_hung_phat, "TIỆM VÀNG KIM HƯNG PHÁT"),
        (scrape_ngoc_thuy, "CÔNG TY TIỆM VÀNG NGỌC THỦY"),
        (scrape_ngoc_tram_q4, "TIỆM VÀNG NGỌC TRÂM QUẬN 4"),
        (scrape_my_ngoc, "TIỆM VÀNG MỸ NGỌC"),
        (scrape_kim_trong_nghia, "CÔNG TY TNHH DỊCH VỤ CẦM ĐỒ KIM TRỌNG NGHĨA"),
        (scrape_van_thong, "CÔNG TY TNHH TM VẠN THÔNG"),
        (scrape_xuan_tung, "CÔNG TY TNHH KINH DOANH VÀNG BẠC ĐÁ QUÍ XUÂN TÙNG"),
    ]

    # 3. Kích hoạt Scrape song song
    scrape_futures = []
    for scrape_func, store_name in scrape_registry:
        logger.info(f"Submitting: {store_name}")
        scrape_futures.append((scrape_func.submit(), store_name))

    # 4. Xử lý kết quả trả về từ các Task Scrape
    upload_futures = []
    for future, store_name in scrape_futures:
        try:
            data = future.result()
            if data:
                # Gửi Task Upload vào hàng chờ (submit) để Flow tiếp tục xử lý các Store khác
                u_future = upload_store_prices.submit(token, store_name, data)
                upload_futures.append(u_future)
            else:
                logger.warning(f"Empty data for {store_name}")
        except Exception as e:
            logger.error(f"Scrape Task Failed for {store_name}: {str(e)}")

    for u_f in upload_futures:
        u_f.wait()

    logger.info("Sync Flow initiated for all stores.")
