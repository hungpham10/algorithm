use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::time::{self, sleep, Duration};

#[actix_rt::main]
pub async fn background_job_client() -> std::io::Result<()> {
    let app = super::Application::new().await;
    let check  = Arc::new(AtomicBool::new(true));
    let timeout = check.clone();
    let update = check.clone();
    
    ctrlc::set_handler(move || {
        update.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let timeout_future = time::sleep(
        Duration::from_secs(
            std::env::var("EXPIRED_TIMEOUT")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(5 * 60),
        ),
    );

    tokio::select! {
        _ = timeout_future => {
            timeout.store(false, Ordering::SeqCst);
        }
        _ = async {
            while check.load(Ordering::SeqCst) {
                sleep(Duration::from_secs(1)).await;
            }
        } => {
        }
    }

    Ok(())
}
