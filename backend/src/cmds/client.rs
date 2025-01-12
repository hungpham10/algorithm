use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::time::{self, Duration};

use crate::schemas::{CronJob, SingleJob, Argument};

#[actix_rt::main]
pub async fn monolith_client() -> std::io::Result<()> {
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
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(5 * 60),
        ),
    );

    // @NOTE: setup cron
    app.start_cron(Vec::new()).await;

    app.perform_job(SingleJob{
        timeout:   5 * 60,
        resolver:  "simulator.setup_new_environment_for_median_strategy".to_string(),
        arguments: Some(vec![
            Argument{
                argument: "resolution".to_string(),
                value:    "1D".to_string(),
            },
            Argument{
                argument: "stock".to_string(),
                value:    "HPG".to_string(),
            },
            Argument{
                argument: "from".to_string(),
                value:    "0".to_string(),
            },
            Argument{
                argument: "to".to_string(),
                value:    "1736637278".to_string(),
            },
        ]),
        from: None,
        to:   None,
    })
    .await;

    tokio::select! {
        _ = timeout_future => {
            timeout.store(false, Ordering::SeqCst);
        }
        _ = async {
            while check.load(Ordering::SeqCst) {
                app.perform_job(SingleJob {
                    timeout:   5 * 60,
                    resolver:  "simulator.perform_training_investors".to_string(),
                    arguments: Some(vec![
                        Argument{
                            argument: "number_of_loop".to_string(),
                            value:    "100".to_string(),
                        },
                        Argument{
                            argument: "mutation_rate".to_string(),
                            value:    "0.1".to_string(),
                        },
                    ]),
                    from: None,
                    to:   None,
                })
                .await;
            }
        } => {
        }
    }

    Ok(())
}
