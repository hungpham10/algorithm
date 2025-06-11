use crate::schemas::CandleStick as OHCL;
use std::cmp::min;

#[inline]
fn cumulate_volume_profile_with_condition(
    candles: &[OHCL],
    number_of_levels: usize,
    overlap: usize,
    condition: fn(&OHCL) -> bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let number_of_days = candles
        .windows(2)
        .filter(|w| w[0].t / 86400 != w[1].t / 86400)
        .count()
        + 1;
    let max_price = candles
        .iter()
        .map(|candle| candle.h)
        .fold(f64::MIN, f64::max);
    let min_price = candles
        .iter()
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
        let volume_per_price = candle.v / price_range;

        if !condition(candle) {
            continue;
        }

        for _ in 0..2 {
            if current == day {
                for level in 0..number_of_levels {
                    let price_level_low = min_price + (level as f64) * price_step;
                    let price_level_high = min_price + ((level + 1) as f64) * price_step;

                    let overlap_start = candle.l.max(price_level_low);
                    let overlap_end = candle.h.min(price_level_high);

                    if overlap_start < overlap_end {
                        for (i, profile) in profiles
                            .iter_mut()
                            .enumerate()
                            .take(number_of_days - overlap + 1)
                        {
                            if i as i64 <= chunk && chunk < (i + overlap) as i64 {
                                profile[level] += volume_per_price * (overlap_end - overlap_start);
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

    (
        profiles,
        (0..number_of_levels)
            .map(|i| min_price + i as f64 * price_step)
            .collect(),
    )
}

#[inline]
pub fn cumulated_volume_profile(
    candles: &[OHCL],
    number_of_levels: usize,
    overlap: usize,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    cumulate_volume_profile_with_condition(candles, number_of_levels, overlap, |_| true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::CandleStick;

    /// Helper to create a CandleStick; open=low, close=high.
    fn make_candle(t: i64, low: f64, high: f64, volume: f64) -> CandleStick {
        CandleStick {
            t: t.try_into().unwrap(),
            o: low,
            h: high,
            l: low,
            c: high,
            v: volume,
        }
    }

    /// Asserts two f64 values are approximately equal.
    fn assert_f64_eq(a: f64, b: f64, eps: f64) {
        assert!(
            (a - b).abs() <= eps,
            "expected {} approx {}, difference {}",
            a,
            b,
            (a - b).abs()
        );
    }

    #[test]
    fn basic_volume_profile() {
        let candles = vec![
            make_candle(0, 1.0, 2.0, 10.0),
            make_candle(1_000, 2.0, 3.0, 20.0),
            make_candle(86_400, 4.0, 5.0, 30.0),
            make_candle(86_400 + 1_000, 5.0, 6.0, 40.0),
        ];
        let (profiles, price_levels) = cumulated_volume_profile(&candles, 4, 1);
        // Two days
        assert_eq!(profiles.len(), 2);
        // Price levels equally spaced from min=1.0 to max=6.0
        let expected_levels = vec![1.0, 2.25, 3.5, 4.75];
        for (a, b) in price_levels.iter().zip(expected_levels.iter()) {
            assert_f64_eq(*a, *b, 1e-6);
        }
        // Day 1 volume = 10 + 20 = 30, Day 2 = 30 + 40 = 70
        let sum0: f64 = profiles[0].iter().sum();
        let sum1: f64 = profiles[1].iter().sum();
        assert_f64_eq(sum0, 30.0, 1e-6);
        assert_f64_eq(sum1, 70.0, 1e-6);
    }

    #[test]
    fn single_day_no_overlap() {
        let candles = vec![
            make_candle(0, 1.0, 2.0, 5.0),
            make_candle(1_000, 2.0, 3.0, 15.0),
        ];
        let (profiles, price_levels) = cumulated_volume_profile(&candles, 3, 0);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].len(), 3);
        assert_eq!(price_levels.len(), 3);
    }

    #[test]
    fn overlap_greater_than_days() {
        let candles = vec![
            make_candle(0, 1.0, 2.0, 5.0),
            make_candle(86_400, 2.0, 3.0, 10.0),
            make_candle(2 * 86_400, 3.0, 4.0, 15.0),
        ];
        // overlap > days should cap to 3 => profiles.len() == 1
        let (profiles, _) = cumulated_volume_profile(&candles, 2, 10);
        assert_eq!(profiles.len(), 1);
    }

    #[test]
    fn empty_candles() {
        use super::cumulate_volume_profile_with_condition as priv_fn;
        let (profiles, levels) = priv_fn(&[], 4, 1, |_| true);
        assert!(profiles.is_empty());
        assert!(levels.is_empty());
    }

    #[test]
    fn only_green_candles() {
        use super::cumulate_volume_profile_with_condition as priv_fn;
        let green = make_candle(0, 1.0, 2.0, 100.0);
        let red = CandleStick {
            t: 0,
            o: 2.0,
            h: 3.0,
            l: 2.0,
            c: 1.0,
            v: 200.0,
        };
        let (profiles, _) = priv_fn(&[green, red], 2, 0, |c| c.c > c.o);
        assert_eq!(profiles.len(), 1);
        // Only green candle contributes to first level
        assert_eq!(profiles[0][0], 100.0);
        assert_eq!(profiles[0][1], 0.0);
    }
}
