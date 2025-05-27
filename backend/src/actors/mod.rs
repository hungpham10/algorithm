use actix::prelude::*;

pub mod tcbs;
pub mod dnse;
pub mod vps;
pub mod cron;
pub mod fireant;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

