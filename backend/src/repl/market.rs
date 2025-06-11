use chrono::{NaiveDate, NaiveDateTime};
use polars::prelude::*;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

use lazy_static::lazy_static;
use std::collections::HashMap;

use crate::actors::dnse::{connect_to_dnse, GetOHCLCommand};
use crate::actors::vps::{
    connect_to_vps, list_of_industry, list_of_vn100, list_of_vn30, GetPriceCommand, Price,
};

lazy_static! {
    static ref INDUSTRY_CODES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("petroleum", "0500");
        m.insert("chemical", "1300");
        m.insert("basic resources", "1700");
        m.insert("construction & building materials", "2300");
        m.insert("industrial goods & services", "2700");
        m.insert("cars & car parts", "3300");
        m.insert("food & beverage", "3500");
        m.insert("personal & household goods", "3700");
        m.insert("medical", "4500");
        m.insert("retail", "5300");
        m.insert("communication", "5500");
        m.insert("travel & entertainment", "5700");
        m.insert("telecomunication", "6500");
        m.insert("electricity, water & petrol", "7500");
        m.insert("banking", "8300");
        m.insert("insurance", "8500");
        m.insert("real estate", "8600");
        m.insert("finance service", "8700");
        m.insert("information technology", "9500");
        m
    };
}

#[pyfunction]
pub fn vn30() -> Vec<String> {
    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async { list_of_vn30().await })
}

#[pyfunction]
pub fn vn100() -> Vec<String> {
    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async { list_of_vn100().await })
}

#[pyfunction]
pub fn sectors() -> Vec<String> {
    INDUSTRY_CODES
        .keys()
        .cloned()
        .map(|k| k.to_string())
        .collect()
}

#[pyfunction]
pub fn industry(name: String) -> Vec<String> {
    actix_rt::Runtime::new().unwrap().block_on(async move {
        if let Some(code) = INDUSTRY_CODES.get(name.as_str()) {
            list_of_industry(code).await
        } else {
            Vec::new()
        }
    })
}

#[pyfunction]
pub fn market(symbols: Vec<String>) -> PyResult<PyDataFrame> {
    let datapoints = actix_rt::Runtime::new()
        .unwrap()
        .block_on(async {
            let actor = connect_to_vps(&symbols);
            actor.send(GetPriceCommand).await.unwrap()
        })
        .map_err(|e| PyRuntimeError::new_err(format!("{:?}", e)))?;

    let mapping: HashMap<String, &Price> = datapoints.iter().map(|p| (p.sym.clone(), p)).collect();

    let mut price_minus1 = Vec::new();
    let mut price_minus2 = Vec::new();
    let mut price_minus3 = Vec::new();
    let mut price_plus1 = Vec::new();
    let mut price_plus2 = Vec::new();
    let mut price_plus3 = Vec::new();

    let mut volume_minus1 = Vec::new();
    let mut volume_minus2 = Vec::new();
    let mut volume_minus3 = Vec::new();
    let mut volume_plus1 = Vec::new();
    let mut volume_plus2 = Vec::new();
    let mut volume_plus3 = Vec::new();

    // Extract data into vectors for each column
    let prices = symbols
        .iter()
        .map(|s| mapping.get(s).unwrap().lastPrice)
        .collect::<Vec<_>>();
    let volumes = symbols
        .iter()
        .map(|s| mapping.get(s).unwrap().lastVolume)
        .collect::<Vec<_>>();
    let change_percent = symbols
        .iter()
        .map(|s| {
            mapping
                .get(s)
                .unwrap()
                .changePc
                .parse::<f64>()
                .unwrap_or(0.0_f64)
        })
        .collect::<Vec<_>>();
    let fb_vols = symbols
        .iter()
        .map(|s| {
            mapping
                .get(s)
                .unwrap()
                .fBVol
                .parse::<f64>()
                .unwrap_or(0.0_f64)
        })
        .collect::<Vec<_>>();
    let fs_vols = symbols
        .iter()
        .map(|s| {
            mapping
                .get(s)
                .unwrap()
                .fSVolume
                .parse::<f64>()
                .unwrap_or(0.0_f64)
        })
        .collect::<Vec<_>>();

    for symbol in &symbols {
        let point = mapping.get(symbol).unwrap();
        let g1 = point.g1.split("|").collect::<Vec<&str>>();
        let g2 = point.g2.split("|").collect::<Vec<&str>>();
        let g3 = point.g3.split("|").collect::<Vec<&str>>();
        let g4 = point.g4.split("|").collect::<Vec<&str>>();
        let g5 = point.g5.split("|").collect::<Vec<&str>>();
        let g6 = point.g6.split("|").collect::<Vec<&str>>();

        price_minus1.push(g4[0].parse::<f64>().unwrap_or(0.0_f64));
        price_minus2.push(g5[0].parse::<f64>().unwrap_or(0.0_f64));
        price_minus3.push(g6[0].parse::<f64>().unwrap_or(0.0_f64));
        price_plus1.push(g1[0].parse::<f64>().unwrap_or(0.0_f64));
        price_plus2.push(g2[0].parse::<f64>().unwrap_or(0.0_f64));
        price_plus3.push(g3[0].parse::<f64>().unwrap_or(0.0_f64));

        volume_minus1.push(g4[1].parse::<f64>().unwrap_or(0_f64));
        volume_minus2.push(g5[1].parse::<f64>().unwrap_or(0_f64));
        volume_minus3.push(g6[1].parse::<f64>().unwrap_or(0_f64));
        volume_plus1.push(g1[1].parse::<f64>().unwrap_or(0_f64));
        volume_plus2.push(g2[1].parse::<f64>().unwrap_or(0_f64));
        volume_plus3.push(g3[1].parse::<f64>().unwrap_or(0_f64));
    }

    // Create series for each column using SmallString for column names
    let df = DataFrame::new(vec![
        Series::new("symbol", &symbols),
        Series::new("price", &prices),
        Series::new("volume", &volumes),
        Series::new("change", &change_percent),
        Series::new("price_minus1", &price_minus1),
        Series::new("price_minus2", &price_minus2),
        Series::new("price_minus3", &price_minus3),
        Series::new("price_plus1", &price_plus1),
        Series::new("price_plus2", &price_plus2),
        Series::new("price_plus3", &price_plus3),
        Series::new("volume_minus1", &volume_minus1),
        Series::new("volume_minus2", &volume_minus2),
        Series::new("volume_minus3", &volume_minus3),
        Series::new("volume_plus1", &volume_plus1),
        Series::new("volume_plus2", &volume_plus2),
        Series::new("volume_plus3", &volume_plus3),
        Series::new("foreign_buy", &fb_vols),
        Series::new("foreign_sell", &fs_vols),
    ])
    .map_err(|e| PyRuntimeError::new_err(format!("Failed to create DataFrame: {}", e)))?;

    Ok(PyDataFrame(df))
}

#[pyfunction]
pub fn price(
    symbol: String,
    resolution: String,
    from: String,
    to: String,
) -> PyResult<PyDataFrame> {
    let from = NaiveDate::parse_from_str(from.as_str(), "%Y-%m-%d")
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse `from`: {}", e)))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| PyRuntimeError::new_err("Invalid time component for `from` date"))?
        .and_utc()
        .timestamp();
    let to = NaiveDate::parse_from_str(to.as_str(), "%Y-%m-%d")
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse `to`: {}", e)))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| PyRuntimeError::new_err("Invalid time component for `to` date"))?
        .and_utc()
        .timestamp();
    let datapoints = actix_rt::Runtime::new()
        .unwrap()
        .block_on(async {
            let actor = connect_to_dnse();

            actor
                .send(GetOHCLCommand {
                    resolution: resolution.clone(),
                    stock: symbol.clone(),
                    from,
                    to,
                })
                .await
                .unwrap()
        })
        .map_err(|e| PyRuntimeError::new_err(format!("{:?}", e)))?;

    Ok(PyDataFrame(
        DataFrame::new(vec![
            Series::new(
                "t",
                datapoints
                    .iter()
                    .map(|it| NaiveDateTime::from_timestamp(it.t.into(), 0))
                    .collect::<Vec<_>>(),
            ),
            Series::new("o", datapoints.iter().map(|it| it.o).collect::<Vec<_>>()),
            Series::new("h", datapoints.iter().map(|it| it.h).collect::<Vec<_>>()),
            Series::new("c", datapoints.iter().map(|it| it.c).collect::<Vec<_>>()),
            Series::new("l", datapoints.iter().map(|it| it.l).collect::<Vec<_>>()),
            Series::new(
                "v",
                datapoints.iter().map(|it| it.v as f64).collect::<Vec<_>>(),
            ),
        ])
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create DataFrame: {:?}", e)))?,
    ))
}
