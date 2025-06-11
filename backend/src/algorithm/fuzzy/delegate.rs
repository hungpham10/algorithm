use std::collections::HashMap;
use std::sync::Arc;

use super::functions::{Add, And, If, Mult, Not, Or, Singleton, Trapezoid, Triangle};
use super::rule::{Function, Rule, RuleError};
use super::{Format, Input};

pub struct Delegate {
    functions: HashMap<String, Arc<dyn Function>>,
}

impl Default for Delegate {
    fn default() -> Self {
        Self::new()
    }
}

impl Delegate {
    pub fn new() -> Delegate {
        let mut functions = HashMap::new();

        // @NOTE: define functions here
        functions.insert(
            "trapezoid".to_string(),
            Arc::new(Trapezoid {}) as Arc<dyn Function>,
        );
        functions.insert(
            "singleton".to_string(),
            Arc::new(Singleton {}) as Arc<dyn Function>,
        );
        functions.insert(
            "triangle".to_string(),
            Arc::new(Triangle {}) as Arc<dyn Function>,
        );
        functions.insert("if".to_string(), Arc::new(If {}) as Arc<dyn Function>);
        functions.insert("and".to_string(), Arc::new(And {}) as Arc<dyn Function>);
        functions.insert("or".to_string(), Arc::new(Or {}) as Arc<dyn Function>);
        functions.insert("not".to_string(), Arc::new(Not {}) as Arc<dyn Function>);
        functions.insert("add".to_string(), Arc::new(Add {}) as Arc<dyn Function>);
        functions.insert("mult".to_string(), Arc::new(Mult {}) as Arc<dyn Function>);

        Delegate { functions }
    }

    pub fn build(&self, expression: &impl Input, format: Format) -> Result<Rule, RuleError> {
        Rule::new(&self.functions, expression, format)
    }

    pub fn default(&self) -> Rule {
        Rule::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // -- Helper ----------------------------------------------------------------------------------
    // Lightweight Input implementation for unit tests.
    // Adjust the trait implementation to match the real `Input` trait signature.
    struct TestInput(String);
    impl Input for TestInput {
        fn as_str(&self) -> &str {
            &self.0
        }
    }

    // -- Tests: Delegate::new --------------------------------------------------------------------
    #[test]
    fn new_registers_all_functions() {
        let delegate = Delegate::new();
        let expected = [
            "trapezoid", "singleton", "triangle", "if", "and", "or", "not", "add", "mult",
        ];
        for name in expected.iter() {
            assert!(
                delegate.functions.contains_key(*name),
                "function `{}` not registered",
                name
            );
        }
        // Sanity-check that each entry is an Arc<dyn Function>.
        for func in delegate.functions.values() {
            // Arc should be clonable and not panic.
            let _clone: Arc<dyn Function> = func.clone();
        }
    }

    // -- Tests: Delegate::build ------------------------------------------------------------------
    #[test]
    fn build_valid_expression_returns_rule() {
        let delegate = Delegate::new();
        let input = TestInput("add(1, 2)".to_string());
        let rule = delegate
            .build(&input, Format::Infix)
            .expect("should build rule for valid expression");
        // If Rule exposes evaluation, uncomment the next line:
        // assert_eq!(rule.evaluate(&[]).unwrap(), 3.0);
        let _ = rule; // suppress unused warnings if evaluate isn’t available
    }

    #[test]
    fn build_with_unknown_function_returns_error() {
        let delegate = Delegate::new();
        let input = TestInput("unknown(1)".to_string());
        let err = delegate
            .build(&input, Format::Infix)
            .expect_err("should error on unknown function");
        matches!(err, RuleError::UnknownFunction(_));
    }

    #[test]
    fn build_with_empty_expression_returns_error() {
        let delegate = Delegate::new();
        let input = TestInput("".to_string());
        assert!(
            delegate.build(&input, Format::Infix).is_err(),
            "empty expression should be invalid"
        );
    }

    // -- Tests: Delegate::default ----------------------------------------------------------------
    #[test]
    fn default_rule_equals_rule_default() {
        let delegate = Delegate::new();
        assert_eq!(delegate.default(), Rule::default());
    }
}
