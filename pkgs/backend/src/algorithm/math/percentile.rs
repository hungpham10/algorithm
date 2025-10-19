pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }

    if n == 1 {
        return sorted[0];
    }

    let p = p.clamp(0.0, 100.0) / 100.0;
    let idx = p * ((n - 1) as f64);
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;

    if lo == hi {
        sorted[lo]
    } else {
        let w = idx - (lo as f64);
        sorted[lo] * (1.0 - w) + sorted[hi] * w
    }
}
