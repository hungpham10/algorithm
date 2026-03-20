use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CandleStick {
    pub t: i32,
    pub o: f64,
    pub h: f64,
    pub c: f64,
    pub l: f64,
    pub v: f64,
}
