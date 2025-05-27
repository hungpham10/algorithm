
pub trait Formular {
    fn formular(&self, index: usize) -> f64;
}

pub struct Difference {
    target: Box<dyn Formular>,
}

impl Difference {
    pub fn new(formular: Box<dyn Formular>) -> Self {
        Difference {
            target: formular,
        }
    }

    pub fn first_difference(&self, index: usize, m: i32) -> Option<f64> {
        let index = index as i32;
        let n_points = 2 * m + 1;
        let denominator = 2.0 * Self::factorial(m).powi(2); // Mẫu số: 2 * (m!)^2
        let mut coefficients = vec![0.0; n_points as usize];

        for k in -m..(m + 1) {
            if k != 0 {
                let numerator = (-1.0_f64).powi(k + 1); // (-1)^(k+1)
                let additional_denom = Self::factorial(m - k.abs()) * Self::factorial(m + k.abs()) * (k as f64);
                coefficients[(k + m) as usize] = (numerator / additional_denom) * Self::factorial(m).powi(2) / denominator;
            }
        }

        let mut sum = 0.0;

        for k in -m..=m {
            sum += coefficients[(k + m) as usize] * self.target.formular((k + index) as usize);
        }

        Some(sum)
    }

    pub fn second_difference(&self, index: usize, m: i32) -> Option<f64> {
        let index = index as i32;
        let n_points = 2 * m + 1;
        let m_factorial = Self::factorial(m) as f64; // Lưu giai thừa của m
        let denominator = 2.0 * Self::factorial(m).powi(2); // Mẫu số: 2 * (m!)^2
        let mut coefficients = vec![0.0; n_points as usize];

        for k in -m..(m + 1) {
            let k_abs = k.abs();
            if k == 0 {
                // Hệ số tại k = 0: -2
                coefficients[(k + m) as usize] = -2.0;
            } else {
                // Hệ số tại k != 0
                let numerator = (-1.0_f64).powi(k_abs) * 2.0 * m_factorial.powi(2);
                let additional_denom = Self::factorial(m - k_abs) as f64 * Self::factorial(m + k_abs) as f64 * (k as f64).powi(2);
                coefficients[(k + m) as usize] = numerator / (additional_denom * denominator);
            }
        }

        let mut sum = 0.0;

        for k in -m..=m {
            sum += coefficients[(k + m) as usize] * self.target.formular((k + index) as usize);
        }

        Some(sum)
    }

    fn factorial(n: i32) -> f64 {
        if n <= 1 {
            return 1.0;
        }
        let mut result = 1.0;
        for i in 2..=n {
            result *= i as f64;
        }
        result
    }
}
