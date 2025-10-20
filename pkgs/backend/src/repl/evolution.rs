use log::debug;
use std::sync::{Arc, Mutex, RwLock};

use polars::datatypes::TimeUnit;
use polars::prelude::*;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

use crate::algorithm::evolution::{Data, Investor, Spot};
use crate::algorithm::genetic::Genetic;
use crate::algorithm::percentile;
use crate::schemas::CandleStick;

#[pyclass]
pub struct Evolution {
    genetic: Option<Arc<Mutex<Genetic<Investor, Spot>>>>,
    candles: Option<Vec<CandleStick>>,
    money: Option<f64>,
    stock: Option<f64>,
    capacity: Option<usize>,
    arguments: Vec<Vec<f64>>,
    pmutation: f64,
    session: i64,
    lifespan: i64,
    stock_holding_period: usize,
    minimum_stock_buy: usize,
}

#[pymethods]
impl Evolution {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self {
            candles: None,
            genetic: None,
            money: None,
            stock: None,
            capacity: None,
            session: 0,
            arguments: Vec::new(),
            pmutation: 0.1,
            lifespan: 10,
            minimum_stock_buy: 100,
            stock_holding_period: 10,
        })
    }

    fn with_minimum_stock_buy(&mut self, value: usize) -> PyResult<()> {
        self.minimum_stock_buy = value;
        Ok(())
    }

    fn with_stock_holding_period(&mut self, value: usize) -> PyResult<()> {
        self.stock_holding_period = value;
        Ok(())
    }

    fn with_capacity(&mut self, capacity: usize) -> PyResult<()> {
        if capacity == 0 {
            return Err(PyRuntimeError::new_err("capacity must be > 0"));
        }
        self.capacity = Some(capacity);
        Ok(())
    }

    fn with_lifespan(&mut self, lifespan: i64) -> PyResult<()> {
        self.lifespan = lifespan;
        Ok(())
    }

    fn with_arguments(&mut self, arguments: Vec<Vec<f64>>) -> PyResult<()> {
        self.arguments = arguments;
        Ok(())
    }

    fn with_money(&mut self, money: f64) -> PyResult<()> {
        self.money = Some(money);
        Ok(())
    }

    fn with_stock(&mut self, stock: f64) -> PyResult<()> {
        self.stock = Some(stock);
        Ok(())
    }

    fn with_candles(&mut self, df: PyDataFrame) -> PyResult<()> {
        let df: DataFrame = df.into();
        let ts_df = df
            .clone()
            .lazy()
            .with_column(
                col("Date")
                    .dt()
                    .timestamp(TimeUnit::Milliseconds)
                    .alias("timestamp"),
            )
            .collect()
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to compute timestamps: {}", error))
            })?;

        let timestamp_series = ts_df
            .column("timestamp")
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to get timestamp column: {}", error))
            })?
            .i64()
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Timestamp column is not i64: {}", error))
            })?
            .clone();

        let open_series = df
            .column("Open")
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to get Open column: {}", error))
            })?
            .f64()
            .map_err(|error| PyRuntimeError::new_err(format!("Open column is not f64: {}", error)))?
            .clone();

        let high_series = df
            .column("High")
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to get High column: {}", error))
            })?
            .f64()
            .map_err(|error| PyRuntimeError::new_err(format!("High column is not f64: {}", error)))?
            .clone();

        let low_series = df
            .column("Low")
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to get Low column: {}", error))
            })?
            .f64()
            .map_err(|error| PyRuntimeError::new_err(format!("Low column is not f64: {}", error)))?
            .clone();

        let close_series = df
            .column("Close")
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to get Close column: {}", error))
            })?
            .f64()
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Close column is not f64: {}", error))
            })?
            .clone();

        let volume_series = df
            .column("Volume")
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Failed to get Volume column: {}", error))
            })?
            .f64()
            .map_err(|error| {
                PyRuntimeError::new_err(format!("Volume column is not f64: {}", error))
            })?
            .clone();

        let len = df.height() as usize;
        let mut candles = Vec::with_capacity(len);

        for i in 0..len {
            let t_opt = timestamp_series.get(i).map(|ts_ms| ts_ms / 1000i64);
            let o_opt = open_series.get(i);
            let h_opt = high_series.get(i);
            let l_opt = low_series.get(i);
            let c_opt = close_series.get(i);
            let v_opt = volume_series.get(i);

            if let (Some(t), Some(o), Some(h), Some(l), Some(c), Some(v)) =
                (t_opt, o_opt, h_opt, l_opt, c_opt, v_opt)
            {
                candles.push(CandleStick {
                    t: t as i32,
                    o,
                    h,
                    l,
                    c,
                    v,
                });
            } else {
                continue;
            }
        }

        if candles.is_empty() {
            return Err(PyRuntimeError::new_err(
                "No valid candles after preprocessing (nulls/invalid dates removed)",
            ));
        }
        self.candles = Some(candles);
        Ok(())
    }

    fn build(&mut self, window: usize) -> PyResult<()> {
        match &self.candles {
            Some(candles) => match self.capacity {
                Some(capacity) => {
                    self.genetic = Some(Arc::new(Mutex::new(Genetic::new(
                        capacity,
                        Arc::new(RwLock::new(
                            Spot::new(
                                Arc::new(RwLock::new(Data::new(Arc::new(candles.clone()), window))),
                                self.money
                                    .ok_or(PyRuntimeError::new_err("Money must be configured"))?,
                                self.stock.unwrap_or(0.0),
                                self.lifespan,
                                self.stock_holding_period,
                                self.minimum_stock_buy,
                            )
                            .map_err(|error| {
                                PyRuntimeError::new_err(format!("Failed to new model: {}", error))
                            })?,
                        )),
                        None,
                    ))));
                    self.session = 0;
                    Ok(())
                }
                None => Err(PyRuntimeError::new_err("Missing capacity")),
            },
            None => Err(PyRuntimeError::new_err("Missing candles")),
        }
    }

    fn fit(
        &mut self,
        n_step: usize,
        n_try: usize,
        n_break: usize,
        n_loop: usize,
        birth_rate: f64,
        shuttle_rate: f64,
    ) -> PyResult<PyDataFrame> {
        match self.capacity {
            Some(capacity) => {
                let mut genetic = self.genetic.as_ref().unwrap().lock().map_err(|error| {
                    PyRuntimeError::new_err(format!("Failed to lock genetic: {}", error))
                })?;
                let mut p95 = Vec::new();
                let mut p75 = Vec::new();
                let mut p55 = Vec::new();
                let mut n = 0;
                let mut done = false;
                let mut step_cnt = 0;
                let mut try_cnt = 0;
                let mut breaking_cnt = 0;
                let mut previous_p75 = 0.0;
                let mut previous_diff_p75 = 0.0;

                if self.session == 0 {
                    genetic
                        .initialize(capacity, self.session, Some(shuttle_rate))
                        .map_err(|error| {
                            PyRuntimeError::new_err(format!("Failed to initialize: {}", error))
                        })?;
                }

                while step_cnt < n_step && !done {
                    for i in 0..n_try {
                        if n_loop > 0 {
                            done = n >= n_loop;

                            if done {
                                break;
                            } else {
                                n += 1;
                            }
                        }

                        genetic
                            .evolute(
                                ((capacity as f64) * birth_rate) as usize,
                                self.session + (step_cnt + i + 1) as i64,
                                self.pmutation,
                            )
                            .map_err(|error| {
                                PyRuntimeError::new_err(format!("Failed to evolute: {}", error))
                            })?;

                        let stats = genetic
                            .statistic(self.session + (step_cnt + i + 1) as i64)
                            .map_err(|error| {
                                PyRuntimeError::new_err(format!(
                                    "Failed to calculate statistic: {}",
                                    error
                                ))
                            })?;
                        let current_p75 = stats.p75;
                        let current_diff_p75 = current_p75 - previous_p75;

                        if current_p75 <= previous_p75 {
                            breaking_cnt += 1;
                        } else if current_diff_p75 <= previous_diff_p75 {
                            breaking_cnt += 1;
                        } else {
                            breaking_cnt = 0;
                        }

                        debug!(
                            "[{}/{}] best={}, p99={}, p95={}, p75={}, p55={}, worst={}",
                            self.session + (step_cnt + i + 1) as i64,
                            self.session + (step_cnt as i64),
                            stats.best,
                            stats.p99,
                            stats.p95,
                            stats.p75,
                            stats.p55,
                            stats.worst,
                        );

                        if breaking_cnt > n_break {
                            break;
                        }

                        try_cnt += 1;
                        previous_p75 = current_p75;
                        previous_diff_p75 = current_diff_p75;

                        if i + 1 < n_try {
                            genetic
                                .fluctuate(
                                    self.session + (step_cnt + i + 1) as i64,
                                    &self.arguments,
                                    self.pmutation,
                                )
                                .map_err(|error| {
                                    PyRuntimeError::new_err(format!(
                                        "Failed to fluctuate: {}",
                                        error
                                    ))
                                })?;
                        }
                    }

                    if try_cnt >= n_try {
                        step_cnt += 1;
                    }

                    let mut fitnesses = genetic.optimize().map_err(|error| {
                        PyRuntimeError::new_err(format!("Failed to optimize: {}", error))
                    })?;

                    if try_cnt >= n_try || done {
                        fitnesses
                            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                        p95.push(percentile(&fitnesses, 95.0));
                        p75.push(percentile(&fitnesses, 75.0));
                        p55.push(percentile(&fitnesses, 55.0));
                    }

                    genetic
                        .initialize(capacity, self.session + step_cnt as i64, Some(shuttle_rate))
                        .map_err(|error| {
                            PyRuntimeError::new_err(format!("Failed to reinitialize: {}", error))
                        })?;

                    try_cnt = 0;
                    breaking_cnt = 0;
                    previous_p75 = 0.0;
                    previous_diff_p75 = 0.0;
                }

                self.session += step_cnt as i64;
                Ok(PyDataFrame(
                    DataFrame::new(vec![
                        Series::new("p95", &p95),
                        Series::new("p75", &p75),
                        Series::new("p55", &p55),
                    ])
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to create DataFrame: {}", e))
                    })?,
                ))
            }
            None => Err(PyRuntimeError::new_err("Capacity is missing")),
        }
    }
}
