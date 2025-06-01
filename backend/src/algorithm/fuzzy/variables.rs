use super::RuleError;
use std::collections::{HashMap, VecDeque};

pub struct Variables {
    buffer_size: usize,
    variables: HashMap<String, VecDeque<f64>>,
}

impl Variables {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            variables: HashMap::new(),
        }
    }

    pub fn create(&mut self, name: &String) -> Result<(), RuleError> {
        if self.variables.contains_key(name) {
            return Err(RuleError {
                message: format!("Variable {} already exists", name),
            });
        }

        self.variables
            .insert(name.clone(), VecDeque::with_capacity(self.buffer_size));
        Ok(())
    }

    pub fn update(&mut self, name: &String, value: f64) -> Result<usize, RuleError> {
        let buffer = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        // Remove oldest value if buffer is full
        if buffer.len() >= self.buffer_size {
            buffer.pop_back();
        }

        // Add new value at front (most recent)
        buffer.push_front(value);
        Ok(buffer.len())
    }

    pub fn get_by_expr(&self, expr: &str) -> Result<f64, RuleError> {
        // Parse expression like "variable[index]"
        let parts: Vec<&str> = expr.split('[').collect();
        if parts.len() != 2 {
            return Err(RuleError {
                message: format!("Invalid expression format: {}", expr),
            });
        }

        let name = parts[0];
        let index_str = parts[1].trim_end_matches(']');
        let index = index_str.parse::<usize>().map_err(|_| RuleError {
            message: format!("Invalid index: {}", index_str),
        })?;

        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer.get(index).copied().ok_or_else(|| RuleError {
            message: format!("Index {} out of bounds for variable {}", index, name),
        })
    }

    pub fn get_by_index(&self, name: &str, index: usize) -> Result<f64, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer.get(index).copied().ok_or_else(|| RuleError {
            message: format!("Index {} out of bounds for variable {}", index, name),
        })
    }

    pub fn list(&self) -> Vec<String> {
        self.variables.keys().cloned().collect()
    }

    pub fn clear(&mut self, name: &str) -> Result<(), RuleError> {
        let buffer = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        buffer.clear();
        Ok(())
    }

    pub fn len(&self, name: &str) -> Result<usize, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        Ok(buffer.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_creation() {
        let mut vars = Variables::new(5);
        assert!(vars.create("test".to_string()).is_ok());
        assert!(vars.create("test".to_string()).is_err());
    }

    #[test]
    fn test_variable_update() {
        let mut vars = Variables::new(3);
        vars.create(&"test".to_string()).unwrap();

        assert_eq!(vars.update(&"test".to_string(), 1.0).unwrap(), 1);
        assert_eq!(vars.update(&"test".to_string(), 2.0).unwrap(), 2);
        assert_eq!(vars.update(&"test".to_string(), 3.0).unwrap(), 3);
        assert_eq!(vars.update(&"test".to_string(), 4.0).unwrap(), 3);

        assert_eq!(vars.get_by_expr("test[0]").unwrap(), 4.0);
        assert_eq!(vars.get_by_expr("test[1]").unwrap(), 3.0);
        assert_eq!(vars.get_by_expr("test[2]").unwrap(), 2.0);
    }

    #[test]
    fn test_variable_get() {
        let mut vars = Variables::new(2);
        vars.create("test".to_string()).unwrap();
        vars.update("test".to_string(), 1.0).unwrap();

        assert!(vars.get_by_expr("invalid").is_err());
        assert!(vars.get_by_expr("test[2]").is_err());
        assert!(vars.get_by_expr("test[invalid]").is_err());
        assert_eq!(vars.get_by_expr("test[0]").unwrap(), 1.0);
    }

    #[test]
    fn test_get_by_index() {
        let mut vars = Variables::new(3);
        vars.create("test".to_string()).unwrap();

        vars.update("test".to_string(), 1.0).unwrap();
        vars.update("test".to_string(), 2.0).unwrap();

        // Test successful cases
        assert_eq!(vars.get_by_index("test", 0).unwrap(), 2.0);
        assert_eq!(vars.get_by_index("test", 1).unwrap(), 1.0);

        // Test error cases
        assert!(vars.get_by_index("nonexistent", 0).is_err());
        assert!(vars.get_by_index("test", 5).is_err());
    }
}
