use super::rule::{Function, Rule, RuleError};
use std::collections::HashMap;

pub struct Noop {}

impl Function for Noop {
    fn evaluate(&self, _: &Rule, _: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(0.0)
    }
}

pub struct Assign {}

impl Function for Assign {
    fn evaluate(&self, rule: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        if pins.len() < 2 {
            Err(RuleError {
                message: "Assign function requires at least 2 arguments".to_string(),
            })
        } else {
            let mapping = HashMap::from([(pins[0].0.clone(), pins[1].1)]);

            if rule.reload(&mapping) == mapping.len() {
                Ok(pins[1].1)
            } else {
                Err(RuleError {
                    message: format!("Fail to assign {}", pins[0].0),
                })
            }
        }
    }
}

pub struct Add {}

impl Function for Add {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        if pins.len() < 2 {
            return Err(RuleError {
                message: "Add function requires at least 2 arguments".to_string(),
            });
        }
        Ok(pins[0].1 + pins[1].1)
    }
}

pub struct Mult {}

impl Function for Mult {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        if pins.len() < 2 {
            return Err(RuleError {
                message: "Mult function requires at least 2 arguments".to_string(),
            });
        }
        Ok(pins[0].1 * pins[1].1)
    }
}

pub struct If {}

impl Function for If {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(if pins[0].0 == pins[1].0 {
            pins[2].1
        } else {
            0.0
        })
    }
}

pub struct And {}

impl Function for And {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(if pins[0].1 < pins[1].1 {
            pins[0].1
        } else {
            pins[1].1
        })
    }
}

pub struct Or {}

impl Function for Or {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(if pins[0].1 > pins[1].1 {
            pins[0].1
        } else {
            pins[1].1
        })
    }
}

pub struct Not {}

impl Function for Not {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(1.0 - pins[0].1)
    }
}

pub struct Singleton {}

impl Function for Singleton {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(if pins[0].1 == pins[1].1 { 1.0 } else { 0.0 })
    }
}

pub struct Trapezoid {}

impl Function for Trapezoid {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok((pins[0].1 - pins[1].1) / (pins[2].1 - pins[3].1))
    }
}

pub struct Triangle {}

impl Function for Triangle {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        if pins[0].1 < pins[1].1 || pins[0].1 > pins[3].1 {
            return Ok(0.0);
        }

        if pins[0].1 <= pins[2].1 {
            Ok((pins[0].1 - pins[1].1) / (pins[2].1 - pins[1].1))
        } else {
            Ok((pins[0].1 - pins[2].1) / (pins[3].1 - pins[2].1))
        }
    }
}
