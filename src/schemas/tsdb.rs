use influxdb::InfluxDbWriteable;
use chrono::{DateTime, Utc};

#[derive(InfluxDbWriteable)]
#[allow(non_snake_case)]
pub struct Order {
    // @NOTE: clock
    pub time: DateTime<Utc>,

    // @NOTE: price
    pub PricePlus1:  f64,
    pub PricePlus2:  f64,
    pub PricePlus3:  f64,
    pub PriceMinus1: f64,
    pub PriceMinus2: f64,
    pub PriceMinus3: f64,

    // @NOTE: price
    pub VolumePlus1:  i64,
    pub VolumePlus2:  i64,
    pub VolumePlus3:  i64,
    pub VolumeMinus1: i64,
    pub VolumeMinus2: i64,
    pub VolumeMinus3: i64,
}

