#[cfg(not(feature = "python"))]
use async_graphql::Object;

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

#[cfg(not(feature = "python"))]
#[Object]
impl CandleStick {
    async fn timestamp(&self) -> i32 {
        self.t
    }

    async fn open(&self) -> f64 {
        self.o
    }

    async fn high(&self) -> f64 {
        self.h
    }

    async fn low(&self) -> f64 {
        self.l
    }

    async fn close(&self) -> f64 {
        self.c
    }

    async fn volume(&self) -> f64 {
        self.v
    }
}
