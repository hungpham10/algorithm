use actix::prelude::*;

use std::error::Error;
use std::fmt;

pub mod cron;
pub mod dnse;
pub mod fireant;
pub mod tcbs;
pub mod vps;

const FUZZY_TRIGGER_THRESHOLD: f64 = 1.0;

#[derive(Debug, Clone)]
pub struct ActorError {
    pub message: String,
}

impl fmt::Display for ActorError {
    /// Formats the `ActorError` by displaying its message.
    ///
    /// # Examples
    ///
    /// ```
    /// let err = ActorError { message: "An error occurred".to_string() };
    /// assert_eq!(format!("{}", err), "An error occurred");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ActorError {}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct UpdateStocksCommand {
    pub stocks: Vec<String>,
}

#[derive(Message)]
#[rtype(result = "Result<f64, ActorError>")]
pub struct GetVariableCommand {
    pub symbol: String,
    pub variable: String,
}
