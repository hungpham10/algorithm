use std::cmp::min;
use crate::schemas::CandleStick as OHCL;

#[inline]
fn order_pressure(candle: &OHCL) -> f64 {
    // @NOTE:
    // boolean logic 
    // f = (candle.c - candle.l) > (candle.h - candle.c) -> up
    // f = (candle.c - candle.l) <= (candle.h - candle.c) -> down
    // fuzzy logic
    // f == true -> fz > 0.5
    // f == false -> fz <= 0.5
    return (candle.c - candle.l) / ((candle.c - candle.l) + (candle.h - candle.c))
}

#[inline]
fn cumulate_volume_profile_with_condition(
    candles: &[OHCL], 
    number_of_levels: usize,
    overlap: usize,
    condition: fn(&OHCL) -> bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let number_of_days = candles.windows(2)
        .filter(|w| w[0].t / 86400 != w[1].t / 86400)
        .count() + 1;
    let max_price = candles.iter()
        .map(|candle| candle.h)
        .fold(f64::MIN, f64::max);
    let min_price = candles.iter()
        .map(|candle| candle.l)
        .fold(f64::MAX, f64::min);
    let price_step = (max_price - min_price) / number_of_levels as f64;
    let overlap = min(overlap, number_of_days);
    let mut profiles = vec![vec![0.0; number_of_levels]; number_of_days - overlap + 1];
    let mut chunk = -1;
    let mut current = 0;

    for candle in candles {
        let day = candle.t / 86400;
        let price_range = candle.h - candle.l;
        let volume_per_price = candle.v as f64 / price_range;

        if !condition(candle) {
            continue;
        }

        for _ in 0..2 {
            if current == day {
                for level in 0..number_of_levels {
                    let price_level_low = min_price + (level as f64) * price_step;
                    let price_level_high = min_price + ((level + 1)  as f64) * price_step;

                    let overlap_start = candle.l.max(price_level_low);
                    let overlap_end = candle.h.min(price_level_high);

                    if overlap_start < overlap_end {
                        for i in 0..(number_of_days - overlap + 1) {
                            if i as i64 <= chunk && chunk < (i + overlap) as i64 {
                                profiles[i][level as usize] += volume_per_price * (overlap_end - overlap_start);
                            }
                        }
                    }
                }

                break;
            } else {
                chunk += 1;
                current = day;
            }
        }
    }

    return (profiles, (0..number_of_levels).map(|i| min_price + i as f64 * price_step).collect());
}

#[inline]
pub fn cumulated_volume_profile(
    candles: &[OHCL], 
    number_of_levels: usize,
    overlap: usize,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    return cumulate_volume_profile_with_condition(candles, number_of_levels, overlap, |_| true);
}
