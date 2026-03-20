use criterion::{Criterion, black_box, criterion_group, criterion_main};

use std::sync::Arc;
use std::time::Duration;

use integration::QueryCandleSticks;
use reqwest::Client as HttpClient;
use tokio::runtime::Runtime;

fn bench_get_ohlc_real_internet(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Arc::new(HttpClient::new());

    // Khởi tạo service
    let service = Arc::new(QueryCandleSticks::new(client, 1000).unwrap());

    // Cấu hình tham số thực tế
    let provider = "ssi";
    let stock = "FPT";
    let res = "1D";

    // Thiết lập thời gian: lấy nến trong vòng 7 ngày gần nhất
    let to = 1739150000; // Hoặc dùng timestamp hiện tại
    let from = to - (7 * 24 * 60 * 60);

    let mut group = c.benchmark_group("Real_Internet_OHLC");

    // QUAN TRỌNG: Giảm sample_size để không bị sàn block IP
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("get_ohlc_warm_cache", |b| {
        b.to_async(&rt).iter(|| {
            // Không xóa timer, nó sẽ hit cache sau lần fetch đầu tiên
            black_box(service.get_candlesticks(provider, stock, res, from, to, 50))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_get_ohlc_real_internet);
criterion_main!(benches);
