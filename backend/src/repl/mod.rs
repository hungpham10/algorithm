use pyo3::prelude::*;

mod analytics;
mod datastore;
mod market;
mod monitor;

#[pymodule]
fn vnscope(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<monitor::Monitor>()?;
    m.add_class::<datastore::Datastore>()?;

    m.add_function(wrap_pyfunction!(analytics::filter, m)?)?;
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
    Ok(())
}
