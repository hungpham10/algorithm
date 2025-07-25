use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

mod analytics;
mod datastore;
mod market;
mod monitor;

#[pymodule]
fn core(_: Python, m: &PyModule) -> PyResult<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    m.add_class::<monitor::Monitor>()?;
    m.add_class::<datastore::Datastore>()?;

    m.add_function(wrap_pyfunction!(analytics::filter, m)?)?;
    m.add_function(wrap_pyfunction!(market::heatmap, m)?)?;
    m.add_function(wrap_pyfunction!(market::order, m)?)?;
    m.add_function(wrap_pyfunction!(market::profile, m)?)?;
    m.add_function(wrap_pyfunction!(market::history, m)?)?;
    m.add_function(wrap_pyfunction!(market::price, m)?)?;
    m.add_function(wrap_pyfunction!(market::market, m)?)?;
    m.add_function(wrap_pyfunction!(market::futures, m)?)?;
    m.add_function(wrap_pyfunction!(market::cw, m)?)?;
    m.add_function(wrap_pyfunction!(market::vn30, m)?)?;
    m.add_function(wrap_pyfunction!(market::vn100, m)?)?;
    m.add_function(wrap_pyfunction!(market::sectors, m)?)?;
    m.add_function(wrap_pyfunction!(market::industry, m)?)?;
    m.add_function(wrap_pyfunction!(configure, m)?)?;
    Ok(())
}

#[pyfunction]
fn configure(table: String, key: String, value: String) -> PyResult<()> {
    match table.as_str() {
        "PROFILE_RESOLUTION" => market::configure_profile_resolution(&key, &value),
        "PRICE_PROVIDER" => market::configure_price_provider(&key),
        _ => Err(PyRuntimeError::new_err(format!(
            "Not support table {}",
            table
        ))),
    }
}
