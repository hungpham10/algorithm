use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};

use serde::{Deserialize, Serialize};

use super::functions::Noop;
use super::input::Input;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Json,
    Expression,

    #[cfg(feature = "python")]
    Python,
}

pub trait Function {
    fn evaluate(&self, rule: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError>;
}

pub struct Rule {
    optree: ExprTree,
}

#[derive(Debug, Clone)]
pub struct RuleError {
    pub message: String,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct ExprTree {
    op: Arc<dyn Function>,
    slots: Vec<bool>,
    thresholds: Vec<bool>,
    nodes: Vec<ExprTree>,
    labels: Vec<String>,
    mapping: HashMap<String, usize>,
    values: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "python", derive(FromPyObject))]
pub struct Pin {
    name: String,
    value: Option<f64>,
    nested: Option<Expression>,
    threshold: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "python", derive(FromPyObject))]
pub struct Expression {
    operator: String,
    pins: Vec<Pin>,
}

impl ExprTree {
    fn evaluate(&self, rule: &Rule) -> Result<f64, RuleError> {
        let mut arguments = Vec::new();
        let mut inode = 0;
        let mut ivalue = 0;

        for (iarg, slot) in self.slots.iter().enumerate() {
            let label = self.labels.get(iarg).unwrap();

            let value = if *slot {
                self.nodes[inode].evaluate(rule)?
            } else {
                self.values[ivalue]
            };

            arguments.push((label.to_string(), value));

            if *slot {
                inode += 1;
            } else {
                ivalue += 1;
            }
        }

        self.op.evaluate(rule, arguments)
    }

    fn labels(&self, is_threshold: bool) -> Vec<&String> {
        let mut ret = Vec::new();
        let mut inode = 0;
        let mut ivalue = 0;

        for (iarg, slot) in self.slots.iter().enumerate() {
            let label = self.labels.get(iarg).unwrap();

            if *slot {
                ret.extend(self.nodes[inode].labels(is_threshold));
            } else if self.thresholds[ivalue] == is_threshold {
                ret.push(label);
            }

            if *slot {
                inode += 1;
            } else {
                ivalue += 1;
            }
        }
        ret
    }

    fn inputs(&self) -> Vec<&String> {
        self.labels(false)
    }

    fn update(&mut self, name: &String, value: f64) -> bool {
        if let Some(index) = self.mapping.get(name) {
            let mut inode = 0;

            for (iarg, slot) in self.slots.iter().enumerate() {
                if inode == *index && self.labels.get(iarg).unwrap() == name {
                    self.values[*index] = value;
                    return true;
                }

                if *slot {
                    inode += 1;
                }
            }
        }

        false
    }
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            optree: ExprTree {
                op: Arc::new(Noop {}),
                slots: Vec::new(),
                thresholds: Vec::new(),
                nodes: Vec::new(),
                labels: Vec::new(),
                values: Vec::new(),
                mapping: HashMap::new(),
            },
        }
    }
}

impl Rule {
    pub fn new<T: Input>(
        functions: &HashMap<String, Arc<dyn Function>>,
        input: &T,
        format: Format,
    ) -> Result<Self, RuleError> {
        match format {
            Format::Json => {
                if let Some(expression) = input.as_json() {
                    Self::from_json(functions, expression)
                } else {
                    Err(RuleError {
                        message: "".to_string(),
                    })
                }
            }
            Format::Expression => {
                if let Some(expression) = input.as_expression() {
                    Self::from_expression(functions, expression)
                } else {
                    Err(RuleError {
                        message: "".to_string(),
                    })
                }
            }

            #[cfg(feature = "python")]
            Format::Python => {
                if let Some(expression) = input.as_python() {
                    Self::from_pydict(functions, expression)
                } else {
                    Err(RuleError {
                        message: "".to_string(),
                    })
                }
            }
        }
        .map(|optree| Self { optree })
    }

    pub fn reload(&mut self, inputs: &HashMap<String, f64>) -> usize {
        let mut cnt = 0;

        for (label, value) in inputs {
            if self.optree.update(label, *value) {
                cnt += 1;
            }
        }

        cnt
    }

    pub fn inputs(&self) -> Vec<&String> {
        self.optree.inputs()
    }

    pub fn evaluate(&self) -> Result<f64, RuleError> {
        self.optree.evaluate(self)
    }

    fn build_expression_nested_tree(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &Expression,
    ) -> Result<ExprTree, RuleError> {
        if functions.contains_key(&expression.operator) {
            let mut output = ExprTree {
                op: functions.get(&expression.operator).unwrap().clone(),
                slots: Vec::new(),
                thresholds: Vec::new(),
                nodes: Vec::new(),
                values: Vec::new(),
                labels: Vec::new(),
                mapping: HashMap::new(),
            };

            for pin in &expression.pins {
                output.mapping.insert(pin.name.clone(), output.slots.len());

                match &pin.nested {
                    Some(nested) => {
                        output.slots.push(true);

                        match Self::build_expression_nested_tree(functions, nested) {
                            Ok(tree) => output.nodes.push(tree),
                            Err(error) => return Err(error),
                        }
                    }
                    None => {
                        output.slots.push(false);

                        if let Some(value) = pin.value {
                            output.values.push(value);
                            output.thresholds.push(false);
                        } else if let Some(value) = pin.threshold {
                            output.values.push(value);
                            output.thresholds.push(true);
                        } else {
                            return Err(RuleError {
                                message: "Pin value is missing".to_string(),
                            });
                        }
                    }
                }

                output.labels.push(pin.name.clone());
            }

            Ok(output)
        } else {
            Err(RuleError {
                message: "Not implemented".to_string(),
            })
        }
    }

    #[cfg(feature = "python")]
    fn build_pydict_nested_tree(
        functions: &HashMap<String, Arc<dyn Function>>,
        pydict: &PyDict,
    ) -> Result<ExprTree, RuleError> {
        let mut op = None;
        let mut slots = Vec::new();
        let mut nodes = Vec::new();
        let mut values = Vec::new();
        let mut thresholds = Vec::new();
        let mut labels = Vec::new();
        let mut mapping = HashMap::new();

        for item in pydict.items() {
            let (key, value): (&PyAny, &PyAny) = item.extract().map_err(|error| RuleError {
                message: format!("Failed to extract data from pydict: {:?}", error),
            })?;

            let key = key
                .str()
                .map_err(|error| RuleError {
                    message: format!("Failed to extract key: {:?}", error),
                })?
                .to_string();

            match key.as_str() {
                "operator" => {
                    let operator = value.extract::<String>().map_err(|error| RuleError {
                        message: format!("Failed to extract operator: {:?}", error),
                    })?;

                    if functions.contains_key(&operator) {
                        op = Some(functions.get(&operator).unwrap().clone());
                    }
                }
                "pins" => {
                    let pypins: &PyList =
                        value.downcast::<PyList>().map_err(|error| RuleError {
                            message: format!("'pins' is not a list: {:?}", error),
                        })?;

                    for item in pypins {
                        let pydict: &PyDict =
                            item.downcast::<PyDict>().map_err(|error| RuleError {
                                message: format!("Pin item is not a dict: {:?}", error),
                            })?;

                        let name: String = pydict
                            .get_item("name")
                            .map_err(|error| RuleError {
                                message: format!("Failed to extract 'name': {:?}", error),
                            })?
                            .ok_or_else(|| RuleError {
                                message: "Missing 'name' key in pin".to_string(),
                            })?
                            .extract()
                            .map_err(|error| RuleError {
                                message: format!("Failed to extract 'name': {:?}", error),
                            })?;

                        mapping.insert(name.clone(), slots.len());

                        if let Some(pynested) =
                            pydict.get_item("nested").map_err(|error| RuleError {
                                message: format!("Failed to extract `nested`: {:?}", error),
                            })?
                        {
                            // Handle nested dictionary
                            let nested_dict: &PyDict =
                                pynested.downcast::<PyDict>().map_err(|error| RuleError {
                                    message: format!("'nested' is not a dict: {:?}", error),
                                })?;
                            nodes.push(Self::build_pydict_nested_tree(functions, nested_dict)?);
                            slots.push(true);
                        } else if let Some(pyvalue) =
                            pydict.get_item("value").map_err(|error| RuleError {
                                message: format!("Failed to get 'value' item: {:?}", error),
                            })?
                        {
                            // Handle value
                            let value: f64 = pyvalue.extract().map_err(|error| RuleError {
                                message: format!("Failed to extract 'value': {:?}", error),
                            })?;
                            values.push(value);
                            slots.push(false);
                            thresholds.push(false);
                        } else if let Some(pyvalue) =
                            pydict.get_item("threshold").map_err(|error| RuleError {
                                message: format!("Failed to get 'threshold' item: {:?}", error),
                            })?
                        {
                            // Handle value
                            let value: f64 = pyvalue.extract().map_err(|error| RuleError {
                                message: format!("Failed to extract 'threshold': {:?}", error),
                            })?;
                            values.push(value);
                            slots.push(false);
                            thresholds.push(true);
                        } else {
                            // Handle with default value
                            values.push(0.0);
                            slots.push(false);
                            thresholds.push(false);
                        }

                        labels.push(name);
                    }
                }
                _ => {
                    return Err(RuleError {
                        message: format!("Unknown key '{}' in expression dictionary", key),
                    });
                }
            }
        }

        if let Some(op) = op {
            Ok(ExprTree {
                op,
                slots,
                nodes,
                labels,
                values,
                thresholds,
                mapping,
            })
        } else {
            Err(RuleError {
                message: format!("cannot build the provided pydict"),
            })
        }
    }

    fn from_json(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &str,
    ) -> Result<ExprTree, RuleError> {
        match serde_json::from_str::<Expression>(expression) {
            Ok(expression) => {
                if functions.contains_key(&expression.operator) {
                    Self::build_expression_nested_tree(functions, &expression)
                } else {
                    Err(RuleError {
                        message: "Not implemented".to_string(),
                    })
                }
            }
            Err(error) => Err(RuleError {
                message: error.to_string(),
            }),
        }
    }

    pub fn from_expression(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &Expression,
    ) -> Result<ExprTree, RuleError> {
        Self::build_expression_nested_tree(functions, expression)
    }

    #[cfg(feature = "python")]
    pub fn from_pydict(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &Py<PyDict>,
    ) -> Result<ExprTree, RuleError> {
        Python::with_gil(|py| Self::build_pydict_nested_tree(functions, expression.as_ref(py)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::collections::HashMap;

    struct TestInput {
        json: Option<String>,
        expr: Option<Expression>,
    }
    impl Input for TestInput {
        fn as_json(&self) -> Option<&String> { self.json.as_ref() }
        fn as_expression(&self) -> Option<&Expression> { self.expr.as_ref() }
    }

    struct AlwaysOne;
    impl Function for AlwaysOne {
        fn evaluate(&self, _rule: &Rule, _pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
            Ok(1.0)
        }
    }
    struct Sum;
    impl Function for Sum {
        fn evaluate(&self, _rule: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
            Ok(pins.into_iter().map(|(_, v)| v).sum())
        }
    }

    fn make_functions() -> HashMap<String, Arc<dyn Function>> {
        let mut map = HashMap::new();
        map.insert("one".into(), Arc::new(AlwaysOne));
        map.insert("sum".into(), Arc::new(Sum));
        map
    }

    #[test]
    fn test_rule_evaluate_expression_happy_path() {
        let expr = Expression {
            operator: "one".into(),
            pins: vec![Pin {
                name: "x".into(),
                value: Some(2.0),
                nested: None,
                threshold: None,
            }],
        };
        let input = TestInput { json: None, expr: Some(expr.clone()) };
        let rule = Rule::new(&make_functions(), &input, Format::Expression).expect("build rule");
        assert_eq!(rule.evaluate().unwrap(), 1.0);
    }

    #[test]
    fn test_rule_evaluate_json_happy_path() {
        let json = serde_json::json!({
            "operator": "one",
            "pins": [
                { "name": "x", "value": 5.0 }
            ]
        }).to_string();
        let input = TestInput { json: Some(json), expr: None };
        let rule = Rule::new(&make_functions(), &input, Format::Json).expect("build rule");
        assert_eq!(rule.evaluate().unwrap(), 1.0);
    }

    #[test]
    fn test_rule_error_missing_pin_value() {
        let expr = Expression {
            operator: "one".into(),
            pins: vec![Pin {
                name: "z".into(),
                value: None,
                nested: None,
                threshold: None,
            }],
        };
        let input = TestInput { json: None, expr: Some(expr) };
        let err = Rule::new(&make_functions(), &input, Format::Expression).unwrap_err();
        assert_eq!(err.message, "Pin value is missing");
    }

    #[test]
    fn test_rule_reload_updates_values() {
        // sum with two pins
        let expr = Expression {
            operator: "sum".into(),
            pins: vec![
                Pin { name: "a".into(), value: Some(1.0), nested: None, threshold: None },
                Pin { name: "b".into(), value: Some(2.0), nested: None, threshold: None },
            ],
        };
        let input = TestInput { json: None, expr: Some(expr) };
        let mut rule = Rule::new(&make_functions(), &input, Format::Expression).unwrap();
        // update only "a"
        let mut updates = HashMap::new();
        updates.insert("a".to_string(), 10.0);
        assert_eq!(rule.reload(&updates), 1);
        assert_eq!(rule.evaluate().unwrap(), 12.0); // 10 + 2
    }

    #[test]
    fn test_rule_inputs_returns_expected_labels() {
        let expr = Expression {
            operator: "sum".into(),
            pins: vec![
                Pin { name: "t1".into(), value: Some(1.0), nested: None, threshold: None },
                Pin { name: "thr".into(), value: None, nested: None, threshold: Some(0.5) },
            ],
        };
        let input = TestInput { json: None, expr: Some(expr) };
        let rule = Rule::new(&make_functions(), &input, Format::Expression).unwrap();
        let labels = rule.inputs();
        assert_eq!(labels, vec![&"t1".to_string()]);
    }
}