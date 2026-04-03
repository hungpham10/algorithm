use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct CandleStick {
    pub t: i32,
    pub o: f64,
    pub h: f64,
    pub c: f64,
    pub l: f64,
    pub v: f64,
}
