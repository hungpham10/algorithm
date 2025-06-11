use pyo3::prelude::*;
use pyo3::types::PyModule;

// Adjust path according to crate name; assuming `backend` is the root crate.
use backend::repl::vnscope;

/// Helper to check if a Python module has the given attribute.
fn has_attr(py: Python<'_>, m: &PyModule, name: &str) -> bool {
    m.getattr(name).is_ok()
}

#[test]
fn test_module_initializes() {
    pyo3::prepare_freethreaded_python!();
    Python::with_gil(|py| {
        let m = PyModule::new(py, "vnscope").expect("Failed to create PyModule");
        assert!(
            vnscope(py, m).is_ok(),
            "vnscope initialization should not error"
        );
    });
}

#[test]
fn test_exports_exist() {
    pyo3::prepare_freethreaded_python!();
    Python::with_gil(|py| {
        let m = PyModule::new(py, "vnscope").unwrap();
        vnscope(py, m).unwrap();
        let expected = [
            "Monitor",
            "Datastore",
            "filter",
            "order",
            "price",
            "market",
            "vn30",
            "vn100",
            "sectors",
            "industry",
        ];
        for sym in expected {
            assert!(
                has_attr(py, m, sym),
                "Expected symbol `{}` not found in module",
                sym
            );
        }
    });
}