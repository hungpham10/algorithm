use polars::prelude::*; // Use polars_core for DataFrame and Series
use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

use crate::actors::tcbs::{connect_to_tcbs, GetOrderCommand};
use crate::algorithm::Variables;

#[pyfunction]
pub fn order(symbol: String) -> PyResult<PyDataFrame> {
    let datapoints = actix_rt::Runtime::new().unwrap().block_on(async {
        let mut datapoints = Vec::new();
        let actor = connect_to_tcbs(
            &[symbol.clone()],
            "".to_string(),
            Arc::new(Mutex::new(Variables::default())),
        );

        for i in 0..10000 {
            let block = actor.send(GetOrderCommand { page: i }).await.unwrap();
            let resp = block.first().unwrap();

            let data = resp.data.clone();

            if resp.numberOfItems == 0 {
                break;
            }

            datapoints.extend(data);
        }

        datapoints
    });

    let df = DataFrame::new(vec![
        Series::new("p", datapoints.iter().map(|d| d.p).collect::<Vec<f64>>()),
        Series::new(
            "v",
            datapoints
                .iter()
                .map(|d| d.v)
                .map(|v| v as f64)
                .collect::<Vec<f64>>(),
        ),
        Series::new("cp", datapoints.iter().map(|d| d.cp).collect::<Vec<f64>>()),
        Series::new(
            "rcp",
            datapoints.iter().map(|d| d.rcp).collect::<Vec<f64>>(),
        ),
        Series::new(
            "a",
            datapoints
                .iter()
                .map(|d| d.a.clone())
                .collect::<Vec<String>>(),
        ),
        Series::new("ba", datapoints.iter().map(|d| d.ba).collect::<Vec<f64>>()),
        Series::new("sa", datapoints.iter().map(|d| d.sa).collect::<Vec<f64>>()),
        Series::new("hl", datapoints.iter().map(|d| d.hl).collect::<Vec<bool>>()),
        Series::new(
            "pcp",
            datapoints.iter().map(|d| d.pcp).collect::<Vec<f64>>(),
        ),
        Series::new(
            "t",
            datapoints
                .iter()
                .map(|d| d.t.clone())
                .collect::<Vec<String>>(),
        ),
    ])
    .map_err(|e| PyRuntimeError::new_err(format!("Failed to create DataFrame: {}", e)))?;

    Ok(PyDataFrame(df))
}
