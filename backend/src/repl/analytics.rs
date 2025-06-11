use polars::prelude::*;
use std::collections::HashMap;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_polars::PyDataFrame;

use crate::algorithm::{Delegate, Format, Rule, Variables};

#[pyfunction]
/// Filters rows in a Polars DataFrame based on a rule and returns the indices of matching rows.
///
/// Converts a Python Polars DataFrame and a rule (provided as a Python dictionary) into Rust types,
/// evaluates the rule for each row, and returns the indices of rows where the rule condition is met.
///
/// # Parameters
/// - `df`: The input DataFrame to filter, provided as a Python Polars DataFrame.
/// - `rule`: The rule to evaluate, provided as a Python dictionary in Python format.
/// - `memory_size`: The memory size to allocate for variable storage during rule evaluation.
///
/// # Returns
/// A vector of row indices (`Vec<u32>`) where the rule condition is satisfied.
///
/// # Errors
/// Returns a Python runtime error if rule construction or evaluation fails.
///
/// # Examples
///
/// ```
/// use pyo3::types::PyDict;
/// let df = ...; // PyDataFrame from Python
/// let rule = ...; // Py<PyDict> representing the rule
/// let indices = filter(df, rule, 1024)?;
/// assert!(indices.len() <= df.height());
/// ```
pub fn filter(df: PyDataFrame, rule: Py<PyDict>, memory_size: usize) -> PyResult<Vec<f64>> {
    let df: DataFrame = df.into();
    let mut rule = Delegate::new()
        .build(&rule, Format::Python)
        .map_err(|e| PyRuntimeError::new_err(format!("Invalid rule: {}", e)))?;

    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async move { filter_in_async(&df, &mut rule, memory_size).await })
}

#[inline]
async fn filter_in_async(
    df: &DataFrame,
    rule: &mut Rule,
    memory_size: usize,
) -> PyResult<Vec<f64>> {
    let mut ret = Vec::new();
    let mut vars = Variables::new(memory_size, 0);
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
    let default = "default".to_string();

    for col in data.keys() {
        vars.create(col).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to create variable {}: {}", col, e))
        })?;
    }

    for irow in 0..df.height() {
        let mut inputs = HashMap::new();

        for (col, vals) in data.iter() {
            vars.update(&default, col, vals[irow]).await.map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to update variable {}: {}", col, e))
            })?;
        }

        for label in rule.inputs() {
            inputs.insert(
                label.to_string(),
                vars.get_by_expr(label).map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to get variable {}: {}", label, e))
                })?,
            );
        }

        rule.reload(&inputs);

        ret.push(
            rule.evaluate()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to evaluate rule: {}", e)))?,
        );
    }

    // Filter DataFrame using take
    Ok(ret)
}
