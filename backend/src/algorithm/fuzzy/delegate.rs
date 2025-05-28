use std::collections::HashMap;
use std::sync::Arc;

use super::functions::{And, If, Not, Or, Singleton, Trapezoid, Triangle};
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

        Delegate { functions }
    }

    pub fn build(&self, expression: &impl Input, format: Format) -> Result<Rule, RuleError> {
        Rule::new(&self.functions, expression, format)
    }
}
