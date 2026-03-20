use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dotenvy::dotenv;
use std::sync::Arc;
use tokio::runtime::Runtime;

use models::cache::Cache;
use models::resolver::Resolver;
use models::secret::Secret;

async fn run_stress_test(resolver: Arc<Resolver>, concurrency: usize, ops_per_worker: usize) {
    let mut handles = Vec::new();
    let tenant_id = 99;

    for worker_id in 0..concurrency {
        let res = Arc::clone(&resolver);
        let handle = tokio::spawn(async move {
            let cache = Cache::new(res, tenant_id);
            for i in 0..ops_per_worker {
                let key = format!("bench_{}_{}", worker_id, i);
                let val = "{\"data\": \"payload\"}".to_string();

                let _ = cache.set(&key, &val, 300).await;
                let _ = cache.get(&key).await;
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }
}

fn bench_cache_system(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    dotenv().ok();

    let resolver = rt.block_on(async {
        let secret = Arc::new(Secret::new().await.unwrap());
        Arc::new(
            Resolver::new(secret)
                .await
                .expect("Failed to init resolver"),
        )
    });

    let mut group = c.benchmark_group("Cache_Stress_Test");

    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let concurrency_levels = vec![1, 4, 8];
    let ops_per_worker = 2;

    for &concurrency in &concurrency_levels {
        group.bench_with_input(
            BenchmarkId::new("Concurrent_Ops", concurrency),
            &concurrency,
            |b, &con| {
                b.to_async(&rt).iter_custom(|iters| {
                    let res = Arc::clone(&resolver);
                    async move {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            run_stress_test(Arc::clone(&res), con, ops_per_worker).await;
                        }
                        start.elapsed()
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_cache_system);
criterion_main!(benches);
