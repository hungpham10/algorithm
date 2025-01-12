use crate::schemas::CandleStick as OHCL;


pub fn find_reverse_points(candles: &[OHCL]) -> Vec<(i32, f64)> {
    let mut reverse_points = Vec::new();
    for i in 1..candles.len() - 1 {
        if (candles[i].o > candles[i - 1].o && candles[i].o > candles[i + 1].o) ||
           (candles[i].o < candles[i - 1].o && candles[i].o < candles[i + 1].o) {
            reverse_points.push((candles[i].t, candles[i].o));
        }
    }
    reverse_points
}