use polars::prelude::*;
use std::collections::HashMap;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_polars::PyDataFrame;

use crate::actors::price::{connect_to_price, GetOHCLCommand};
use crate::actors::vps::{connect_to_vps, GetPriceCommand, Price};
use crate::actors::{list_cw, list_futures, list_of_industry, list_of_vn100, list_of_vn30};
use crate::algorithm::fuzzy::{Delegate, Format, Rule, Variables};

#[pyfunction]
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
                vars.get_by_selected_expr(label).map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;
    use pyo3::prelude::*;
    use pyo3::types::PyDict;
    use pyo3_polars::PyDataFrame;
    use std::collections::HashMap;
    use tokio;

    #[derive(Default)]
    struct TestRule;

    impl TestRule {
        fn inputs(&self) -> Vec<&str> {
            Vec::new()
        }

        fn reload(&mut self, _vars: &HashMap<String, f64>) {}

        fn evaluate(&self) -> Result<f64, String> {
            Ok(1.0)
        }
    }

    async fn build_test_df() -> DataFrame {
        let s1 = Series::new("col1", &[1.0_f64, 2.0, 3.0]);
        let s2 = Series::new("col2", &[4.0_f64, 5.0, 6.0]);
        DataFrame::new(vec![s1, s2]).unwrap()
    }

    #[tokio::test]
    async fn test_filter_in_async_happy_path() {
        let df = build_test_df().await;
        let mut rule = TestRule::default();
        let res = filter_in_async(&df, &mut rule, 8).await.expect("ok");
        assert_eq!(res.len(), df.height());
        assert!(res.iter().all(|v| *v == 1.0));
    }

    #[derive(Default)]
    struct DummyRule;

    impl DummyRule {
        fn inputs(&self) -> Vec<&str> {
            Vec::new()
        }

        fn reload(&mut self, _vars: &HashMap<String, f64>) {}

        fn evaluate(&self) -> Result<f64, String> {
            Err("boom".to_string())
        }
    }

    #[tokio::test]
    async fn test_filter_in_async_evaluate_error() {
        let df = build_test_df().await;
        let mut rule = DummyRule::default();
        let res = filter_in_async(&df, &mut rule, 8).await;
        assert!(matches!(res, Err(e) if e.to_string().contains("Failed to evaluate rule")));
    }

    // Override Delegate build to simulate construction error
    #[cfg(test)]
    mod delegate_override {
        use crate::algorithm::{Delegate, Format};
        use pyo3::types::PyDict;

        impl Delegate {
            pub fn build(&mut self, _dict: &PyDict, _fmt: Format) -> Result<&mut Self, String> {
                Err("bad".to_string())
            }
        }
    }

    #[test]
    fn test_filter_delegate_error() {
        Python::with_gil(|py| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let df = rt.block_on(build_test_df());
            let py_df = PyDataFrame::new(py, df).unwrap();
            let dict = PyDict::new(py);
            let res = filter(py_df, dict.into(), 8);
            assert!(res.is_err());
            let err = res.unwrap_err();
            assert!(err.to_string().contains("Invalid rule"));
        });
    }
}
