use chrono::{NaiveDate, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use polars::prelude::*;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

use std::collections::HashMap;
use std::sync::Mutex;

use crate::actors::dnse::{connect_to_dnse, GetOHCLCommand};
use crate::actors::tcbs::{connect_to_tcbs, GetOrderCommand};
use crate::actors::vps::{connect_to_vps, GetPriceCommand, Price};
use crate::actors::{list_cw, list_futures, list_of_industry, list_of_vn100, list_of_vn30};
use crate::algorithm::cumulate_volume_profile;
use crate::algorithm::fuzzy::Variables;
use crate::schemas::CandleStick;

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
pub fn futures() -> Vec<String> {
    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async { list_futures().await })
}

#[pyfunction]
pub fn cw() -> PyResult<PyDataFrame> {
    let datapoints = actix_rt::Runtime::new()
        .unwrap()
        .block_on(async { list_cw().await });

    let last_trading_time_parsed = datapoints
        .iter()
        .map(|it| {
            let day: i32 = it.last_trading_date.as_str()[6..8].parse().unwrap();
            let year: i32 = it.last_trading_date.as_str()[0..4].parse().unwrap();
            let month: i32 = it.last_trading_date.as_str()[4..6].parse().unwrap();

            (day, month, year)
        })
        .collect::<Vec<_>>();

    Ok(PyDataFrame(
        DataFrame::new(vec![
            Series::new(
                "symbol",
                datapoints
                    .iter()
                    .map(|it| it.symbol.clone())
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "underlying",
                datapoints
                    .iter()
                    .map(|it| it.underlying.clone())
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "year",
                last_trading_time_parsed
                    .iter()
                    .map(|it| it.2)
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "month",
                last_trading_time_parsed
                    .iter()
                    .map(|it| it.1)
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "day",
                last_trading_time_parsed
                    .iter()
                    .map(|it| it.0)
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "last_trading_date",
                datapoints
                    .iter()
                    .map(|it| it.last_trading_date.clone())
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "exercise_price",
                datapoints
                    .iter()
                    .map(|it| it.exercise_price)
                    .collect::<Vec<_>>(),
            ),
            Series::new(
                "exercise_ratio",
                datapoints
                    .iter()
                    .map(|it| {
                        let parts = it.exercise_ratio.split(':').collect::<Vec<&str>>();

                        parts[0].parse::<f64>().unwrap()
                    })
                    .collect::<Vec<_>>(),
            ),
        ])
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create DataFrame: {:?}", e)))?,
    ))
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
    let lots = symbols
        .iter()
        .map(|s| mapping.get(s).unwrap().lot as f64)
        .collect::<Vec<_>>();
    let change_percent = symbols
        .iter()
        .map(|s| {
            let c = mapping.get(s).unwrap().lastPrice;
            let r = mapping.get(s).unwrap().r;

            if c > r {
                mapping
                    .get(s)
                    .unwrap()
                    .changePc
                    .parse::<f64>()
                    .unwrap_or(0.0_f64)
            } else {
                -mapping
                    .get(s)
                    .unwrap()
                    .changePc
                    .parse::<f64>()
                    .unwrap_or(0.0_f64)
            }
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

        price_plus1.push(g4[0].parse::<f64>().unwrap_or(0.0_f64));
        price_plus2.push(g5[0].parse::<f64>().unwrap_or(0.0_f64));
        price_plus3.push(g6[0].parse::<f64>().unwrap_or(0.0_f64));
        price_minus1.push(g1[0].parse::<f64>().unwrap_or(0.0_f64));
        price_minus2.push(g2[0].parse::<f64>().unwrap_or(0.0_f64));
        price_minus3.push(g3[0].parse::<f64>().unwrap_or(0.0_f64));

        volume_plus1.push(g4[1].parse::<f64>().unwrap_or(0_f64));
        volume_plus2.push(g5[1].parse::<f64>().unwrap_or(0_f64));
        volume_plus3.push(g6[1].parse::<f64>().unwrap_or(0_f64));
        volume_minus1.push(g1[1].parse::<f64>().unwrap_or(0_f64));
        volume_minus2.push(g2[1].parse::<f64>().unwrap_or(0_f64));
        volume_minus3.push(g3[1].parse::<f64>().unwrap_or(0_f64));
    }

    // Create series for each column using SmallString for column names
    let df = DataFrame::new(vec![
        Series::new("symbol", &symbols),
        Series::new("price", &prices),
        Series::new("lot", &lots),
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

#[pyfunction]
pub fn profile(
    symbols: Vec<String>,
    resolution: String,
    lookback: i64,
    number_of_levels: usize,
) -> PyResult<PyDataFrame> {
    let mut columns = vec![Series::new("symbol", symbols.clone())];
    let to = Utc::now().timestamp();
    let from = match resolution.as_str() {
        "1D" => Ok(to - 24 * 60 * 60 * lookback),
        "1W" => Ok(to - 7 * 24 * 60 * 60 * lookback),
        _ => Err(PyRuntimeError::new_err(format!(
            "Not support resolution `{}`",
            resolution
        ))),
    }?;

    let datapoints = symbols
        .iter()
        .map(|symbol| {
            actix_rt::Runtime::new().unwrap().block_on(async {
                let actor = connect_to_dnse();
                let candles = actor
                    .send(GetOHCLCommand {
                        resolution: match resolution.as_str() {
                            "1D" => "1H".to_string(),
                            "1W" => "4H".to_string(),
                            _ => "1D".to_string(),
                        },
                        stock: symbol.clone(),
                        from,
                        to,
                    })
                    .await
                    .unwrap()
                    .unwrap();
                let (profiles, levels) = cumulate_volume_profile(&candles, number_of_levels, 0);

                (
                    profiles[0].clone(),
                    levels.first().cloned().unwrap_or(0.0),
                    levels.last().cloned().unwrap_or(0.0),
                )
            })
        })
        .collect::<Vec<(Vec<f64>, f64, f64)>>();

    let profile_length = datapoints
        .get(0)
        .map(|(profile, _, _)| profile.len())
        .unwrap_or(0);

    for i in 0..profile_length {
        let col_name = format!("level_{}", i);
        let values = datapoints
            .iter()
            .map(|(profile, _, _)| profile.get(i).cloned().unwrap_or(0.0))
            .collect::<Vec<f64>>();
        columns.push(Series::new(&col_name, values));
    }

    columns.push(Series::new(
        "price_at_level_first",
        datapoints
            .iter()
            .map(|(_, first, _)| *first)
            .collect::<Vec<f64>>(),
    ));
    columns.push(Series::new(
        "price_at_level_last",
        datapoints
            .iter()
            .map(|(_, _, last)| *last)
            .collect::<Vec<f64>>(),
    ));

    Ok(PyDataFrame(DataFrame::new(columns).map_err(|e| {
        PyRuntimeError::new_err(format!("Failed to create DataFrame: {}", e))
    })?))
}

#[pyfunction]
pub fn history(symbols: Vec<String>, resolution: String, lookback: i64) -> PyResult<PyDataFrame> {
    let to = Utc::now().timestamp();
    let from = match resolution.as_str() {
        "1D" => Ok(to - 24 * 60 * 60 * lookback),
        "1W" => Ok(to - 7 * 24 * 60 * 60 * lookback),
        _ => Err(PyRuntimeError::new_err(format!(
            "Not support resolution `{}`",
            resolution
        ))),
    }?;

    let datapoints = symbols
        .iter()
        .map(|symbol| {
            actix_rt::Runtime::new().unwrap().block_on(async {
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
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();

    // Find the maximum number of candlesticks across all symbols
    let max_candles = datapoints
        .iter()
        .map(|candles| candles.len())
        .max()
        .unwrap_or(0);
    if max_candles == 0 {
        return Ok(PyDataFrame(
            DataFrame::new(vec![Series::new("symbol", &symbols)]).map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to create DataFrame: {:?}", e))
            })?,
        ));
    }

    // Prepare columns for the DataFrame
    let mut columns = vec![Series::new("symbol", &symbols)];

    // For each candlestick index (day), create columns for o, h, c, l, v
    for i in 0..max_candles {
        let mut o_values = Vec::new();
        let mut h_values = Vec::new();
        let mut c_values = Vec::new();
        let mut l_values = Vec::new();
        let mut v_values = Vec::new();

        for candles in &datapoints {
            if let Some(candle) = candles.get(i) {
                o_values.push(candle.o);
                h_values.push(candle.h);
                c_values.push(candle.c);
                l_values.push(candle.l);
                v_values.push(candle.v);
            } else {
                o_values.push(f64::NAN);
                h_values.push(f64::NAN);
                c_values.push(f64::NAN);
                l_values.push(f64::NAN);
                v_values.push(f64::NAN);
            }
        }

        columns.push(Series::new(&format!("o_day_{}", i + 1), o_values));
        columns.push(Series::new(&format!("h_day_{}", i + 1), h_values));
        columns.push(Series::new(&format!("c_day_{}", i + 1), c_values));
        columns.push(Series::new(&format!("l_day_{}", i + 1), l_values));
        columns.push(Series::new(&format!("v_day_{}", i + 1), v_values));
    }

    // Create DataFrame
    Ok(PyDataFrame(DataFrame::new(columns).map_err(|e| {
        PyRuntimeError::new_err(format!("Failed to create DataFrame: {:?}", e))
    })?))
}

#[pyfunction]
pub fn order(symbol: String) -> PyResult<PyDataFrame> {
    let datapoints = actix_rt::Runtime::new().unwrap().block_on(async {
        let mut datapoints = Vec::new();
        let actor = connect_to_tcbs(
            &[symbol.clone()],
            "".to_string(),
            Arc::new(Mutex::new(Variables::default())),
        )
        .await;

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

    let t = datapoints
        .iter()
        .map(|d| {
            (
                d.t.as_str()[0..2].parse::<i32>().unwrap_or(0),
                d.t.as_str()[3..5].parse::<i32>().unwrap_or(0),
                d.t.as_str()[6..8].parse::<i32>().unwrap_or(0),
            )
        })
        .collect::<Vec<_>>();

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
        Series::new("ba", datapoints.iter().map(|d| d.ba).collect::<Vec<_>>()),
        Series::new("sa", datapoints.iter().map(|d| d.sa).collect::<Vec<_>>()),
        Series::new("hl", datapoints.iter().map(|d| d.hl).collect::<Vec<bool>>()),
        Series::new(
            "pcp",
            datapoints.iter().map(|d| d.pcp).collect::<Vec<f64>>(),
        ),
        Series::new("h", t.iter().map(|i| i.0).collect::<Vec<_>>()),
        Series::new("m", t.iter().map(|i| i.1).collect::<Vec<_>>()),
        Series::new("s", t.iter().map(|i| i.2).collect::<Vec<_>>()),
    ])
    .map_err(|e| PyRuntimeError::new_err(format!("Failed to create DataFrame: {}", e)))?;

    Ok(PyDataFrame(df))
}
