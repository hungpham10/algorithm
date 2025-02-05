use tokio_schedule::{every, Job};
use chrono::Utc;
use std::sync::Arc;
use lib::components::simulator::{
    connect_to_simulator,
    SetupSettingCommand,
    EnableTrainingCommand,
    EvaluateFitnessCommand,
};
use lib::actors::cron::{
    connect_to_cron,
    CronResolver
};
use lib::actors::dnse::{
    connect_to_dnse,
    GetOHCLCommand,
};

#[actix_rt::test]
async fn test_simulator_median_strategy() {
    let mut resolver = CronResolver::new();
    let dnse = Arc::new(connect_to_dnse());
    let simulator = connect_to_simulator(
        &mut resolver,
        dnse.clone(),
        1000,
        10,
        false,
    );
    
    let candles = Arc::new(dnse.send(GetOHCLCommand{
            resolution: String::from("1D"),
            stock:      String::from("PDR"),
            from:       Utc::now().timestamp() - 250*24*60*60,
            to:         Utc::now().timestamp(),
        })
        .await
        .unwrap()
        .unwrap());

    simulator.send(SetupSettingCommand {
        batch_money_for_fund: 100,
        candles: candles,
        orders: None,
        stock: "PDR".to_string(),
        money: 30_000_000.0,
        arg_gen_min: None,
        arg_gen_max: None,
        lookback_order_history: None,
        lookback_candle_history: None,
    })
    .await
    .unwrap();

    simulator.send(EnableTrainingCommand)
        .await
        .unwrap();

    simulator.send(EvaluateFitnessCommand{
        number_of_couple: 10,
        number_of_loop: 100,
        mutation_rate: 0.1,
        mutation_args: Vec::new(),
        number_of_simulator: 1,
    })
    .await
    .unwrap();
}
