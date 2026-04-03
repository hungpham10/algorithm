struct ExponentialMovingAverage {
    period: usize,
    alpha: f64,
    current_ema: Option<f64>,
}

impl ExponentialMovingAverage {
    fn new(period: usize) -> Self {
        Self {
            period,
            alpha: 2.0 / (period as f64 + 1.0),
            current_ema: None,
        }
    }

    fn next(&mut self, value: f64) -> f64 {
        match self.current_ema {
            Some(prev_ema) => {
                let new_ema = self.alpha * value + (1.0 - self.alpha) * prev_ema;
                self.current_ema = Some(new_ema);
                new_ema
            }
            None => {
                // Lần đầu tiên, EMA chính là giá trị đầu vào
                self.current_ema = Some(value);
                value
            }
        }
    }
}
