from prefect.client.schemas.schedules import CronSchedule
from gold import gold_sync_flow

if __name__ == "__main__":
    gold_sync_flow.serve(
        name="gold-price-sync",
        schedule=CronSchedule(
            cron="0 7-22/4 * * 1-5",
            timezone="Asia/Ho_Chi_Minh",  # Đảm bảo chạy đúng giờ VN
        ),
    )
