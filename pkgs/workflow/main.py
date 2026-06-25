from prefect import serve
from prefect.client.schemas.schedules import CronSchedule

from bank_rate import bank_rate_sync_flow
from gold import gold_sync_flow
from shopee import shopee_price_sync_flow

if __name__ == "__main__":
    # ─── Bank Rate Sync (8h sáng T2-T7, giờ VN) ─────────────────────────
    bank_rate_deployment = bank_rate_sync_flow.to_deployment(
        name="bank-rate-sync",
        schedule=CronSchedule(
            cron="0 8 * * 1-6",
            timezone="Asia/Ho_Chi_Minh",
        ),
    )

    # ─── Gold Price Sync (7h-22h mỗi tiếng 1 lần, T2-T7) ───────────────
    gold_deployment = gold_sync_flow.to_deployment(
        name="gold-price-sync",
        schedule=CronSchedule(
            cron="0 7-22/1 * * 1-6",
            timezone="Asia/Ho_Chi_Minh",
        ),
    )

    serve(bank_rate_deployment, gold_deployment)
