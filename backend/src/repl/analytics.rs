use polars::prelude::*;
use std::collections::HashMap;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_polars::PyDataFrame;

use crate::algorithm::{Delegate, Format, Rule, Variables};

const FUZZY_TRIGGER_THRESHOLD: f64 = 1.0;

#[pyfunction]
/// Filters rows of a DataFrame based on a rule and returns the indices of matching rows.
///
/// Converts a Python Polars DataFrame and a rule (provided as a Python dictionary) into Rust types,
/// then asynchronously evaluates the rule for each row. Returns the indices of rows where the rule evaluates to the trigger threshold.
///
/// # Parameters
/// - `df`: The input DataFrame to filter, provided as a Python Polars DataFrame.
/// - `rule`: The rule to evaluate, provided as a Python dictionary in the expected format.
/// - `memory_size`: The memory size to allocate for variable storage during rule evaluation.
///
/// # Returns
/// A vector of row indices (`Vec<u32>`) where the rule evaluates to the trigger threshold.
///
/// # Errors
/// Returns a Python runtime error if rule construction fails or if an error occurs during evaluation.
///
/// # Examples
///
/// ```
/// # use your_crate::filter;
/// let indices = filter(py_df, py_rule, 1024)?;
/// assert!(!indices.is_empty());
/// ```
pub fn filter(df: PyDataFrame, rule: Py<PyDict>, memory_size: usize) -> PyResult<Vec<u32>> {
    let df: DataFrame = df.into();
    let rule = Delegate::new()
        .build(&rule, Format::Python)
        .map_err(|e| PyRuntimeError::new_err(format!("Invalid rule: {}", e)))?;

    actix_rt::Runtime::new()
        .unwrap()
        .block_on(async move { filter_in_async(&df, &rule, memory_size).await })
}

/// Asynchronously evaluates a rule on each row of a DataFrame and returns the indices of rows that match.
///
/// For each row, updates variables with the row's Float64 column values, retrieves rule inputs, and evaluates the rule.
/// If the rule evaluation equals the trigger threshold, the row index is included in the result.
///
/// # Arguments
///
/// * `df` - Reference to the DataFrame to be filtered.
/// * `rule` - The rule to evaluate on each row.
/// * `memory_size` - The memory size used for variable storage.
///
/// # Returns
///
/// A Python result containing a vector of indices for rows where the rule evaluation matches the trigger threshold.
///
/// # Errors
///
/// Returns a Python runtime error if variable creation, update, retrieval, or rule evaluation fails.
///
/// # Examples
///
/// ```
/// # use polars::prelude::*;
/// # use your_crate::{filter_in_async, Rule};
/// # async fn example() -> pyo3::PyResult<()> {
/// let df = DataFrame::new(vec![Series::new("a", &[1.0, 2.0, 3.0])])?;
/// let rule = Rule::from_str("a > 1.5")?;
/// let indices = filter_in_async(&df, &rule, 10).await?;
/// assert_eq!(indices, vec![1, 2]);
/// # Ok(())
/// # }
/// ```
async fn filter_in_async(df: &DataFrame, rule: &Rule, memory_size: usize) -> PyResult<Vec<u32>> {
    let mut selected_indices = Vec::new();
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

    for col in data.keys() {
        vars.create(col).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to create variable {}: {}", col, e))
        })?;
    }

    for irow in 0..df.height() {
        let mut inputs = HashMap::new();

        for (col, vals) in data.iter() {
            vars.update(col, vals[irow]).await.map_err(|e| {
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
