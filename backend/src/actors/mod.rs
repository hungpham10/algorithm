use actix::prelude::*;

pub mod cron;
pub mod dnse;
pub mod fireant;
pub mod tcbs;
pub mod vps;

const FUZZY_TRIGGER_THRESHOLD: f64 = 1.0;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct UpdateStocksCommand {
    pub stocks: Vec<String>,
}
