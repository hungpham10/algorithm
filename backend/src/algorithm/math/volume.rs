use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::schemas::CandleStick as OHCL;

#[inline]
fn timestamp_to_day(t: i32) -> i32 {
    t / (24 * 60 * 60)
}

#[inline]
pub fn cumulate_volume_range(heatmap: &Vec<Vec<f64>>) -> Result<Vec<(usize, usize, usize)>> {
    let mut centers = BTreeMap::new();

    let sorted = heatmap
        .iter()
        .map(|row| row.iter().sum())
        .collect::<Vec<_>>()
        .iter()
        .enumerate()
        .collect::<Vec<(_, &f64)>>()
        .into_iter()
        .sorted_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(Ordering::Greater))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    for t in sorted {
        let mut found = false;
        let mut left = 0;
        let mut right = 0;
        let mut center = 0;
        let mut weight = 0;

        for (group, (begin, end, order)) in &centers {
            left = *begin;
            right = *end;
            found = t + 1 == *begin || t - 1 == *end;

            if t + 1 == *begin {
                center = *group;
                left = t;
                weight = *order;
                break;
            }
            if t - 1 == *end {
                center = *group;
                right = t;
                weight = *order;
                break;
            }
        }

        if !found {
            if centers.contains_key(&t) {
                return Err(anyhow!("Center {} is not synchronized fully", t));
            }

            left = t;
            right = t;
            center = t;
            weight = centers.len();
        }

        centers.insert(center, (left, right, weight));
    }

    Ok(centers
        .iter()
        .map(|(center, (begin, end, weight))| (*center, *begin, *end, weight))
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|a, b| a.3.partial_cmp(b.3).unwrap_or(Ordering::Greater))
        .map(|(center, begin, end, _)| (center, begin, end))
        .collect::<Vec<_>>())
}

#[inline]
pub fn cumulate_volume_profile(
    candles: &[OHCL],
    number_of_levels: usize,
    overlap_days: usize,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    if candles.is_empty() || number_of_levels == 0 {
        return (vec![], vec![]);
    }

    let mut all_volumes: Vec<Vec<f64>> = Vec::new();
    let mut price_levels: Vec<f64> = Vec::new();

    // Tìm min/max price cho toàn bộ dataset để đảm bảo consistency
    let global_min_price = candles.iter().map(|c| c.l).fold(f64::INFINITY, f64::min);
    let global_max_price = candles
        .iter()
        .map(|c| c.h)
        .fold(f64::NEG_INFINITY, f64::max);

    if global_min_price == f64::INFINITY || global_max_price == f64::NEG_INFINITY {
        return (vec![], vec![]);
    }

    let price_range = global_max_price - global_min_price;
    let price_bin_size = price_range / number_of_levels as f64;

    // Tạo price levels một lần duy nhất
    for i in 0..number_of_levels {
        price_levels.push(global_min_price + i as f64 * price_bin_size);
    }

    // Nếu overlap_days = 0, tính volume profile cho toàn bộ dataset như bt
    if overlap_days == 0 {
        let mut temp_profile: HashMap<usize, f64> = HashMap::new();

        for candle in candles {
            // Tính bin index cho low và high price
            let low_bin = ((candle.l - global_min_price) / price_bin_size).floor() as usize;
            let high_bin = ((candle.h - global_min_price) / price_bin_size).floor() as usize;

            // Đảm bảo bin index trong phạm vi hợp lệ
            let low_bin = low_bin.min(number_of_levels - 1);
            let high_bin = high_bin.min(number_of_levels - 1);

            // Phân bổ đều khối lượng cho các bin trong phạm vi giá của cây nến
            let num_bins = (high_bin - low_bin + 1) as f64;
            let volume_per_bin = if num_bins > 0.0 {
                candle.v / num_bins
            } else {
                candle.v
            };

            // Cộng dồn khối lượng vào các bin
            for bin in low_bin..=high_bin {
                *temp_profile.entry(bin).or_insert(0.0) += volume_per_bin;
            }
        }

        // Chuyển đổi HashMap thành Vec với đúng thứ tự
        let mut volumes_for_all = vec![0.0; number_of_levels];
        for (bin_index, volume) in temp_profile {
            if bin_index < number_of_levels {
                volumes_for_all[bin_index] = volume;
            }
        }

        return (vec![volumes_for_all], price_levels);
    }

    // Tìm các điểm bắt đầu ngày mới
    let mut day_start_indices: Vec<usize> = Vec::new();
    let mut current_day = timestamp_to_day(candles[0].t);
    day_start_indices.push(0);

    for (i, candle) in candles.iter().enumerate().skip(1) {
        let candle_day = timestamp_to_day(candle.t);
        if candle_day > current_day {
            day_start_indices.push(i);
            current_day = candle_day;
        }
    }

    // Nếu không đủ ngày, return empty
    if day_start_indices.len() < overlap_days {
        return (vec![], vec![]);
    }

    // Tính volume profile cho từng window theo ngày
    for window_start in 0..=(day_start_indices.len() - overlap_days) {
        let start_index = day_start_indices[window_start];
        let end_index = if window_start + overlap_days < day_start_indices.len() {
            day_start_indices[window_start + overlap_days]
        } else {
            candles.len()
        };

        let window = &candles[start_index..end_index];

        // Tính volume profile cho window hiện tại
        let mut temp_profile: HashMap<usize, f64> = HashMap::new();

        for candle in window {
            // Tính bin index cho low và high price
            let low_bin = ((candle.l - global_min_price) / price_bin_size).floor() as usize;
            let high_bin = ((candle.h - global_min_price) / price_bin_size).floor() as usize;

            // Đảm bảo bin index trong phạm vi hợp lệ
            let low_bin = low_bin.min(number_of_levels - 1);
            let high_bin = high_bin.min(number_of_levels - 1);

            // Phân bổ đều khối lượng cho các bin trong phạm vi giá của cây nến
            let num_bins = (high_bin - low_bin + 1) as f64;
            let volume_per_bin = if num_bins > 0.0 {
                candle.v / num_bins
            } else {
                candle.v
            };

            // Cộng dồn khối lượng vào các bin
            for bin in low_bin..=high_bin {
                *temp_profile.entry(bin).or_insert(0.0) += volume_per_bin;
            }
        }

        // Chuyển đổi HashMap thành Vec với đúng thứ tự
        let mut volumes_for_window = vec![0.0; number_of_levels];
        for (bin_index, volume) in temp_profile {
            if bin_index < number_of_levels {
                volumes_for_window[bin_index] = volume;
            }
        }

        all_volumes.push(volumes_for_window);
    }

    (all_volumes, price_levels)
}

// Hàm helper để test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cumulate_volume_profile() {
        let candles = vec![
            // Ngày 1 (t: 1000000000 = 2001-09-09)
            OHCL {
                o: 100.0,
                h: 105.0,
                c: 102.0,
                l: 98.0,
                v: 1000.0,
                t: 1000000000,
            },
            OHCL {
                o: 102.0,
                h: 108.0,
                c: 106.0,
                l: 101.0,
                v: 1500.0,
                t: 1000010000,
            },
            // Ngày 2 (t: 1000086400 = 2001-09-10)
            OHCL {
                o: 106.0,
                h: 110.0,
                c: 109.0,
                l: 104.0,
                v: 2000.0,
                t: 1000086400,
            },
            OHCL {
                o: 109.0,
                h: 112.0,
                c: 111.0,
                l: 107.0,
                v: 1200.0,
                t: 1000096400,
            },
            // Ngày 3 (t: 1000172800 = 2001-09-11)
            OHCL {
                o: 111.0,
                h: 115.0,
                c: 113.0,
                l: 109.0,
                v: 1800.0,
                t: 1000172800,
            },
            OHCL {
                o: 113.0,
                h: 118.0,
                c: 116.0,
                l: 112.0,
                v: 2200.0,
                t: 1000182800,
            },
        ];

        // Test với overlap_days = 0 (tính toàn bộ dataset)
        let (volumes_all, levels_all) = cumulate_volume_profile(&candles, 10, 0);
        println!("All data volume profile: {:?}", volumes_all);
        assert_eq!(volumes_all.len(), 1); // Chỉ có 1 profile cho toàn bộ data

        // Test với overlap_days = 2 (sliding window theo ngày)
        let (volumes, levels) = cumulate_volume_profile(&candles, 10, 2);

        println!("Number of windows: {}", volumes.len());
        println!("Price levels: {:?}", levels);
        for (i, volume_profile) in volumes.iter().enumerate() {
            println!("Window {} (2 days): {:?}", i, volume_profile);
        }

        assert_eq!(volumes.len(), 2); // Có 2 windows: ngày [1,2] và ngày [2,3]
    }
}
