use std::fmt;
use std::sync::Arc;
use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use serde::{Deserialize, Serialize};

use super::input::Input;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Json,
    Expression,
    Python,
}

pub trait Function {
    fn evaluate(&self, pins: Vec<(String, f64)>) -> Result<f64, RuleError>;
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

#[derive(Serialize, Deserialize, Debug, Clone, FromPyObject)]
pub struct Pin {
    name:   String,
    value:  Option<f64>,
    nested: Option<Expression>,
}

pub struct ExprTree {
    op:     Arc<dyn Function>,
    slots:  Vec<bool>,
    nodes:  Vec<ExprTree>,
    labels: HashMap<String, usize>,
    values: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromPyObject)]
pub struct Expression {
    operator: String,
    pins:     Vec<Pin>,
}

impl ExprTree {
    fn evaluate(&self) -> Result<f64, RuleError> {
        let mut arguments = Vec::new();
        let mut iarg = 0;
        let mut inode = 0;
        let mut ivalue = 0;

        for slot in self.slots.iter() {
            let label = self.labels.keys()
                .nth(iarg)
                .unwrap();
            let value = if *slot {
                self.nodes[inode].evaluate()?
            } else {
                self.values[ivalue]
            };

            arguments.push((label.to_string(), value));

            if *slot {
                inode += 1;
            } else {
                ivalue += 1;
            }
            iarg += 1;
        }

        self.op.evaluate(arguments)
    }

    fn labels(&self) -> Vec<&String> {
        let mut labels = self.labels.keys().collect::<Vec<_>>();

        for node in &self.nodes {
            labels.extend(node.labels());
        }

        labels
    }

    fn update(&mut self, name: &String, value: f64) -> bool {
        if let Some(index) = self.labels.get(name) {
            let mut iarg = 0;
            let mut inode = 0;

            for slot in &self.slots {
                if inode == *index && self.labels.keys().nth(iarg).unwrap() == name {
                    self.values[*index] = value;
                    return true;
                }

                if *slot {
                    inode += 1;
                }
                iarg += 1;
            }
        }

        false
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
                    Err(RuleError { message: "".to_string() })
                }
            },
            Format::Expression => {
                if let Some(expression) = input.as_expression() {
                    Self::from_expression(functions, expression)
                } else {
                    Err(RuleError { message: "".to_string() })
                }
            },
            Format::Python => {
                if let Some(expression) = input.as_python() {
                    Self::from_pydict(functions, expression)
                } else {
                    Err(RuleError { message: "".to_string() })
                }
            }
        }.map(|optree| Self { optree })
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

    pub fn labels(&self) -> Vec<&String> {
        self.optree.labels()
    }

    pub fn evaluate(&self) -> Result<f64, RuleError> {
        self.optree.evaluate()
    }

    fn build_expression_nested_tree(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &Expression,
    ) -> Result<ExprTree, RuleError> {
        if functions.contains_key(&expression.operator) {
            let mut output = ExprTree { 
                op:     functions.get(&expression.operator).unwrap().clone(), 
                slots:  Vec::new(),
                nodes:  Vec::new(),
                values: Vec::new(),
                labels: HashMap::new(),
            };

            for pin in &expression.pins {

                match &pin.nested {
                    Some(nested) => {
                        output.slots.push(true);
                        output.labels.insert(pin.name.clone(), output.nodes.len());

                        match Self::build_expression_nested_tree(functions, &nested) {
                            Ok(tree) => output.nodes.push(tree),
                            Err(error) => return Err(error),
                        }
                    },
                    None => {
                        output.slots.push(false);
                        output.labels.insert(pin.name.clone(), output.values.len());

                        if let Some(value) = pin.value {
                            output.values.push(value); 
                        } else {
                            return Err(RuleError { message: "Pin value is missing".to_string() });
                        }
                    }
                }
            }

            Ok(output)
        } else {
            Err(RuleError { message: "Not implemented".to_string() })
        }
    }

    fn from_json(
        functions: &HashMap<String, Arc<dyn Function>>, 
        expression: &str
    ) -> Result<ExprTree, RuleError> {
        match serde_json::from_str::<Expression>(expression) {
            Ok(expression) => {
                if functions.contains_key(&expression.operator) {
                    Self::build_expression_nested_tree(functions, &expression)
                } else {
                    Err(RuleError { message: "Not implemented".to_string() })
                }
            },
            Err(error) => Err(RuleError { message: error.to_string() }),
        }
    }

    pub fn from_expression(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &Expression,
    ) -> Result<ExprTree, RuleError> {
        Self::build_expression_nested_tree(functions, expression)
    }

    pub fn from_pydict(
        functions: &HashMap<String, Arc<dyn Function>>,
        expression: &Py<PyDict>,
    ) -> Result<ExprTree, RuleError> {
        Python::with_gil(|py| {
            let pydict = expression.as_ref(py);
            let expression = pydict.extract::<Expression>()
                .map_err(|error| RuleError{
                    message: format!("Failed to extract expresstion: {:?}", error)
                })?;

            Self::build_expression_nested_tree(functions, &expression)
        })
    }
}
