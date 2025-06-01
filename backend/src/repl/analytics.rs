use polars::prelude::*;
use std::collections::HashMap;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_polars::PyDataFrame;

use crate::algorithm::{Delegate, Format, Variables};

const FUZZY_TRIGGER_THRESHOLD: f64 = 1.0;

#[pyfunction]
pub fn filter(
    py: Python,
    df: PyDataFrame,
    rule: Py<PyDict>,
    memory_size: usize,
) -> PyResult<Vec<u32>> {
    let mut df: DataFrame = df.into();
    let mut vars = Variables::new(memory_size);
    let mut rule = Delegate::new()
        .build(&rule, Format::Python)
        .map_err(|e| PyRuntimeError::new_err(format!("Invalid rule: {}", e)))?;
    let mut selected_indices = Vec::new();
    let data: HashMap<String, Vec<f64>> = df
        .get_column_names()
        .into_iter()
        .filter_map(|col| {
            df.column(col)
                .ok()
                .filter(|series| series.dtype() == &DataType::Float64)
                .and_then(|series| {
                    series.f64().ok().map(|ca| {
                        let data = ca
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect::<Vec<f64>>();
                        (col.to_string(), data)
                    })
                })
        })
        .collect();

    for col in data.keys() {
        vars.create(col).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to create variable {}: {}", col, e))
        })?;
    }

    for irow in 0..df.height() {
        let mut inputs = HashMap::new();

        for (col, vals) in data.iter() {
            vars.update(col, vals[irow]).map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to update variable {}: {}", col, e))
            })?;
        }

        for label in rule.labels() {
            inputs.insert(
                label.to_string(),
                vars.get_by_expr(label).map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to get variable {}: {}", label, e))
                })?,
            );
        }

        let result = rule
            .evaluate()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to evaluate rule: {}", e)))?;

        if result == FUZZY_TRIGGER_THRESHOLD {
            selected_indices.push(irow as u32);
        }
    }

    // Filter DataFrame using take
    Ok(selected_indices)
}
