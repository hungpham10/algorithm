use pyo3::prelude::*;

mod datastore;
mod market;
mod monitor;
mod order;

#[pymodule]
fn vnscope(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<monitor::Monitor>()?;
    m.add_class::<datastore::Datastore>()?;

    m.add_function(wrap_pyfunction!(order::order, m)?)?;
    m.add_function(wrap_pyfunction!(market::market, m)?)?;
    m.add_function(wrap_pyfunction!(market::vn30, m)?)?;
    m.add_function(wrap_pyfunction!(market::vn100, m)?)?;
    m.add_function(wrap_pyfunction!(market::sectors, m)?)?;
    m.add_function(wrap_pyfunction!(market::industry, m)?)?;
    Ok(())
}
