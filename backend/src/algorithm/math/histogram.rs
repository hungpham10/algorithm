
pub fn histogram_with_condition<T>(data: &Vec<(f64, T)>, n_bins: usize, condition: fn(&(f64, T)) -> bool) -> Vec<(f64, f64, usize)> {
    let bellow = data.iter().fold(f64::INFINITY, |a, b| a.min(b.0));
    let above = data.iter().fold(-f64::INFINITY, |a, b| a.max(b.0));
    let width = (above - bellow) / n_bins as f64;

    let mut ret = vec![(0f64, 0f64, 0); n_bins + 1];

    for i in 0..n_bins {
        ret[i] = (bellow + i as f64 * width, bellow + (i + 1) as f64 * width, 0);
    }

    for i in 0..data.len() {
        if condition(&data[i]) {
            ret[((data[i].0 - bellow) / width).floor() as usize].2 += 1;
        }
    }

    return ret;
}

pub fn histogram_without_condition(data: &Vec<f64>, n_bins: usize) -> Vec<(f64, f64, usize)> {
    let bellow = data.iter().fold(f64::INFINITY, |a, b| a.min(*b));
    let above = data.iter().fold(-f64::INFINITY, |a, b| a.max(*b));
    let width = (above - bellow) / n_bins as f64;

    let mut ret = vec![(0f64, 0f64, 0); n_bins + 1];

    for i in 0..n_bins {
        ret[i] = (bellow + i as f64 * width, bellow + (i + 1) as f64 * width, 0);
    }

    for i in 0..data.len() {
        ret[((data[i] - bellow) / width).floor() as usize].2 += 1;
    }

    return ret;
}
