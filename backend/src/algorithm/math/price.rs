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

pub fn find_gap_candles(candles: &[OHCL]) -> (Vec<i32>, Vec<i32>) {
    let mut gap_up = Vec::new();
    let mut gap_down = Vec::new();

    for i in 1..candles.len() {
        if candles[i].o > candles[i - 1].h {
            gap_up.push(candles[i].t);
        } else if candles[i].o < candles[i - 1].l {
            gap_down.push(candles[i].t);
        }
    }

    return (gap_up, gap_down);
}

pub fn find_swing_points(candles: &[OHCL], lookback: usize) -> (Vec<usize>, Vec<usize>) {
    let mut swing_highs = Vec::new();
    let mut swing_lows = Vec::new();

    if candles.len() < (2 * lookback + 1) {
        return (swing_highs, swing_lows);
    }

    for i in lookback..(candles.len() - lookback) {
        let is_swing_hig = {
            let before_high: Vec<_> = candles[(i - lookback)..i].iter().map(|c| c.h).collect();
            let after_high: Vec<_> = candles[(i + 1)..(i + lookback + 1)].iter().map(|c| c.h).collect();

            before_high.iter().all(|&x| x < candles[i].h) &&
            after_high.iter().all(|&x| x < candles[i].h)
        };

        if is_swing_hig {
            swing_highs.push(i);
        }

        let is_swing_low = {
            let before_low: Vec<_> = candles[(i - lookback)..i].iter().map(|c| c.l).collect();
            let after_low: Vec<_> = candles[(i + 1)..(i + lookback + 1)].iter().map(|c| c.l).collect();

            before_low.iter().all(|&x| x > candles[i].l) &&
            after_low.iter().all(|&x| x > candles[i].l)
        };
        if is_swing_low {
            swing_lows.push(i);
        }
    }

    return (swing_highs, swing_lows);
}
