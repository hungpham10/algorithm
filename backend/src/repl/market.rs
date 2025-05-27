use polars::prelude::*; // Use polars_core for DataFrame and Series

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3_polars::PyDataFrame;

use std::collections::HashMap;
use lazy_static::lazy_static;

use crate::actors::dnse::connect_to_dnse;
use crate::actors::vps::{
    Price,
    list_of_vn30,
    list_of_vn100,
    list_of_industry,
    connect_to_vps,
    GetPriceCommand,
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
        .block_on(async {
            list_of_vn30().await
        })
}

#[pyfunction]
pub fn vn100() -> Vec<String> {
    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async {
            list_of_vn100().await
        })
}

#[pyfunction]
pub fn sectors() -> Vec<String> {
    INDUSTRY_CODES.keys()
        .cloned()
        .map(|k| k.to_string())
        .collect()
}

#[pyfunction]
pub fn industry(name: String) -> Vec<String> {
    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async move {
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
            actor.send(GetPriceCommand)
                .await
                .unwrap()
        });

    let mapping: HashMap<String, &Price> = datapoints.iter()
        .map(|p| (p.sym.clone(), p))
        .collect();

    let mut price_minus1 = Vec::new(); 
    let mut price_minus2 = Vec::new(); 
    let mut price_minus3 = Vec::new(); 
    let mut price_plus1  = Vec::new(); 
    let mut price_plus2  = Vec::new(); 
    let mut price_plus3  = Vec::new(); 

    let mut volume_minus1 = Vec::new();
    let mut volume_minus2 = Vec::new();
    let mut volume_minus3 = Vec::new();
    let mut volume_plus1  = Vec::new();
    let mut volume_plus2  = Vec::new();
    let mut volume_plus3  = Vec::new();

    // Extract data into vectors for each column
    let prices = symbols.iter()
        .map(|s| mapping.get(s).unwrap().lastPrice)
        .collect::<Vec<_>>();
    let volumes = symbols.iter()
        .map(|s| mapping.get(s).unwrap().lastVolume)
        .collect::<Vec<_>>();
    let change_percent = symbols.iter()
        .map(|s| mapping.get(s).unwrap().changePc.parse::<f64>().unwrap_or(0.0 as f64))
        .collect::<Vec<_>>();
    let fb_vols = symbols.iter()
        .map(|s| mapping.get(s).unwrap().fBVol.parse::<f64>().unwrap_or(0.0 as f64))
        .collect::<Vec<_>>();
    let fs_vols = symbols.iter()
        .map(|s| mapping.get(s).unwrap().fSVolume.parse::<f64>().unwrap_or(0.0 as f64))
        .collect::<Vec<_>>();

    for symbol in &symbols {
        let point = mapping.get(symbol).unwrap();
        let g1 = point.g1.split("|").collect::<Vec<&str>>();
        let g2 = point.g2.split("|").collect::<Vec<&str>>();
        let g3 = point.g3.split("|").collect::<Vec<&str>>();
        let g4 = point.g4.split("|").collect::<Vec<&str>>();
        let g5 = point.g5.split("|").collect::<Vec<&str>>();
        let g6 = point.g6.split("|").collect::<Vec<&str>>();
                    
        price_minus1.push(g4[0].parse::<f64>().unwrap_or(0.0 as f64));
        price_minus2.push(g5[0].parse::<f64>().unwrap_or(0.0 as f64));
        price_minus3.push(g6[0].parse::<f64>().unwrap_or(0.0 as f64));
        price_plus1.push(g1[0].parse::<f64>().unwrap_or(0.0 as f64));
        price_plus2.push(g2[0].parse::<f64>().unwrap_or(0.0 as f64));
        price_plus3.push(g3[0].parse::<f64>().unwrap_or(0.0 as f64));

        volume_minus1.push(g4[1].parse::<f64>().unwrap_or(0 as f64));
        volume_minus2.push(g5[1].parse::<f64>().unwrap_or(0 as f64));
        volume_minus3.push(g6[1].parse::<f64>().unwrap_or(0 as f64));
        volume_plus1.push(g1[1].parse::<f64>().unwrap_or(0 as f64));
        volume_plus2.push(g2[1].parse::<f64>().unwrap_or(0 as f64));
        volume_plus3.push(g3[1].parse::<f64>().unwrap_or(0 as f64));
    }

    // Create series for each column using SmallString for column names
    let df = DataFrame::new(vec![
            Series::new("symbol".into(), &symbols),
            Series::new("price".into(), &prices),
            Series::new("volume".into(), &volumes),
            Series::new("change".into(), &change_percent),
            Series::new("price_minus1".into(), &price_minus1),
            Series::new("price_minus2".into(), &price_minus2),
            Series::new("price_minus3".into(), &price_minus3),
            Series::new("price_plus1".into(), &price_plus1),
            Series::new("price_plus2".into(), &price_plus2),
            Series::new("price_plus3".into(), &price_plus3),
            Series::new("volume_minus1".into(), &volume_minus1),
            Series::new("volume_minus2".into(), &volume_minus2),
            Series::new("volume_minus3".into(), &volume_minus3),
            Series::new("volume_plus1".into(), &volume_plus1),
            Series::new("volume_plus2".into(), &volume_plus2),
            Series::new("volume_plus3".into(), &volume_plus3),
            Series::new("foreign_buy".into(), &fb_vols),
            Series::new("foreign_sell".into(), &fs_vols),
        ])
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create DataFrame: {}", e)))?;

    Ok(PyDataFrame(df))
}
