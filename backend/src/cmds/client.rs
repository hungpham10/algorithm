use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::time::{self, sleep, Duration};

use crate::schemas::{CronJob, SingleJob, Argument};

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

    // @NOTE: setup cron
    app.start_cron(vec![
        CronJob {
            interval:  "* * * * *".to_string(),
            timeout:   5 * 60,
            resolver:  "simulator.perform_training_investors".to_string(),
            arguments: Some(vec![
                Argument{
                    argument: "number_of_loop".to_string(),
                    value:    "10".to_string(),
                },
                Argument{
                    argument: "number_of_simulator".to_string(),
                    value:    "30".to_string(),
                },
                Argument{
                    argument: "mutation_rate".to_string(),
                    value:    "0.01".to_string(),
                },
            ]),
        },
    ]).await;

    app.perform_job(SingleJob{
        timeout:   5 * 60,
        resolver:  "simulator.setup_new_environment_for_median_strategy".to_string(),
        arguments: Some(vec![
            Argument{
                argument: "resolution".to_string(),
                value:    "1D".to_string(),
            },
            Argument{
                argument: "batch_money_for_fund".to_string(),
                value:    "30".to_string(),
            },
            Argument{
                argument: "from".to_string(),
                value:    "1700495220".to_string(),
            },
            Argument{
                argument: "to".to_string(),
                value:    "1736693826".to_string(),
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
                sleep(Duration::from_secs(1)).await;
            }
        } => {
        }
    }

    Ok(())
}
