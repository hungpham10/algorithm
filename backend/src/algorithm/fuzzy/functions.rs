use super::rule::{Function, Rule, RuleError};

pub struct Noop {}

impl Function for Noop {
    fn evaluate(&self, _: &Rule, _: Vec<(String, f64)>) -> Result<f64, RuleError> {
        Ok(0.0)
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
#[cfg(test)]
mod tests {
    use super::*;

    // Import real Rule and RuleError if they are in the super module.
    // If the real Rule is a trait with no required methods, we can implement a dummy struct here.
    struct DummyRule;
    impl Rule for DummyRule {}

    // Helper for extracting f64 from Ok variant
    fn ok(val: Result<f64, RuleError>) -> f64 {
        val.unwrap()
    }

    #[test]
    fn test_noop_always_zero() {
        let func = Noop {};
        let rule = DummyRule;
        let result = ok(func.evaluate(&rule, vec![]));
        assert!((result - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_add_success() {
        let func = Add {};
        let rule = DummyRule;
        let result = ok(func.evaluate(&rule, vec![
            ("a".into(), 2.0),
            ("b".into(), 3.5),
        ]));
        assert!((result - 5.5).abs() < 1e-6);
    }

    #[test]
    fn test_add_insufficient_args() {
        let func = Add {};
        let rule = DummyRule;
        let err = func.evaluate(&rule, vec![("a".into(), 1.0)]).unwrap_err();
        assert!(err.message.contains("requires at least 2"));
    }

    #[test]
    fn test_mult_success() {
        let func = Mult {};
        let rule = DummyRule;
        let result = ok(func.evaluate(&rule, vec![
            ("a".into(), 4.0),
            ("b".into(), 2.5),
        ]));
        assert!((result - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_mult_insufficient_args() {
        let func = Mult {};
        let rule = DummyRule;
        let err = func.evaluate(&rule, vec![("a".into(), 1.0)]).unwrap_err();
        assert!(err.message.contains("requires at least 2"));
    }

    #[test]
    fn test_if_match_and_no_match() {
        let func = If {};
        let rule = DummyRule;
        let yes = ok(func.evaluate(&rule, vec![
            ("x".into(), 0.0),
            ("x".into(), 0.0),
            ("val".into(), 7.0),
        ]));
        assert!((yes - 7.0).abs() < 1e-6);

        let no = ok(func.evaluate(&rule, vec![
            ("x".into(), 0.0),
            ("y".into(), 0.0),
            ("val".into(), 7.0),
        ]));
        assert!((no - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_and_returns_min() {
        let func = And {};
        let rule = DummyRule;
        let result = ok(func.evaluate(&rule, vec![
            ("a".into(), 0.7),
            ("b".into(), 0.2),
        ]));
        assert!((result - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_or_returns_max() {
        let func = Or {};
        let rule = DummyRule;
        let result = ok(func.evaluate(&rule, vec![
            ("a".into(), 0.7),
            ("b".into(), 0.2),
        ]));
        assert!((result - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_not_inverse() {
        let func = Not {};
        let rule = DummyRule;
        let res1 = ok(func.evaluate(&rule, vec![("a".into(), 0.0)]));
        assert!((res1 - 1.0).abs() < 1e-6);
        let res2 = ok(func.evaluate(&rule, vec![("a".into(), 1.0)]));
        assert!((res2 - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_singleton_equality() {
        let func = Singleton {};
        let rule = DummyRule;
        let equal = ok(func.evaluate(&rule, vec![
            ("a".into(), 5.0),
            ("b".into(), 5.0),
        ]));
        assert!((equal - 1.0).abs() < 1e-6);
        let diff = ok(func.evaluate(&rule, vec![
            ("a".into(), 5.0),
            ("b".into(), 3.0),
        ]));
        assert!((diff - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_trapezoid_ratio() {
        let func = Trapezoid {};
        let rule = DummyRule;
        let result = ok(func.evaluate(&rule, vec![
            ("x".into(), 10.0),  // numerator part 1
            ("y".into(), 2.0),   // numerator part 2
            ("z".into(), 8.0),   // denominator part 1
            ("w".into(), 1.0),   // denominator part 2
        ]));
        // (10 - 2) / (8 - 1) = 8 / 7
        assert!((result - (8.0 / 7.0)).abs() < 1e-6);
    }

    #[test]
    fn test_triangle_values() {
        let func = Triangle {};
        let rule = DummyRule;

        // Outside lower bound
        let outside_low = ok(func.evaluate(&rule, vec![
            ("x".into(), 0.0),
            ("l".into(), 1.0),
            ("p".into(), 2.0),
            ("r".into(), 3.0),
        ]));
        assert!((outside_low - 0.0).abs() < 1e-6);

        // Rising edge
        let rising = ok(func.evaluate(&rule, vec![
            ("x".into(), 1.5),
            ("l".into(), 1.0),
            ("p".into(), 2.0),
            ("r".into(), 3.0),
        ]));
        let expected_rising = (1.5 - 1.0) / (2.0 - 1.0);
        assert!((rising - expected_rising).abs() < 1e-6);

        // Falling edge
        let falling = ok(func.evaluate(&rule, vec![
            ("x".into(), 2.5),
            ("l".into(), 1.0),
            ("p".into(), 2.0),
            ("r".into(), 3.0),
        ]));
        let expected_falling = (2.5 - 2.0) / (3.0 - 2.0);
        assert!((falling - expected_falling).abs() < 1e-6);
    }
}
