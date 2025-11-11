use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::schemas::CandleStick as OHCL;

pub struct VolumeProfile {
    heatmap: Vec<Vec<f64>>,
    levels: Vec<f64>,
    ranges: Vec<(usize, usize, usize)>,
}

impl VolumeProfile {
    pub fn new() -> Self {
        Self {
            heatmap: Vec::new(),
            levels: Vec::new(),
            ranges: Vec::new(),
        }
    }

    pub fn new_from_candles(
        candles: &[OHCL],
        number_of_levels: usize,
        overlap: usize,
        interval_in_hour: i32,
    ) -> Result<Self> {
        let (heatmap, levels) =
            Self::cumulate_volume_profile(candles, number_of_levels, overlap, interval_in_hour)?;
        let ranges = Self::cumulate_volume_range(&heatmap)?;

        Ok(Self {
            heatmap,
            levels,
            ranges,
        })
    }

    pub fn calculate(
        &mut self,
        candles: &[OHCL],
        number_of_levels: usize,
        overlap: usize,
        interval_in_hour: i32,
    ) -> Result<()> {
        let (heatmap, levels) =
            Self::cumulate_volume_profile(candles, number_of_levels, overlap, interval_in_hour)?;

        self.ranges = Self::cumulate_volume_range(&heatmap)?;
        self.levels = levels;
        self.heatmap = heatmap;
        Ok(())
    }

    pub fn heatmap(&self) -> &Vec<Vec<f64>> {
        &self.heatmap
    }

    pub fn levels(&self) -> &Vec<f64> {
        &self.levels
    }

    pub fn ranges(&self) -> &Vec<(usize, usize, usize)> {
        &self.ranges
    }

    pub fn center(&self, index: usize) -> f64 {
        self.levels[self.ranges[index].0]
    }

    pub fn top(&self, index: usize) -> f64 {
        self.levels[self.ranges[index].1]
    }

    pub fn down(&self, index: usize) -> f64 {
        self.levels[self.ranges[index].2]
    }

    pub fn number_of_bias(&self) -> usize {
        self.ranges.len()
    }

    #[inline]
    fn timestamp_to_pin(t: i32, interval_in_hour: i32) -> i32 {
        t / (interval_in_hour * 60 * 60)
    }

    #[inline]
    pub fn cumulate_volume_range(heatmap: &Vec<Vec<f64>>) -> Result<Vec<(usize, usize, usize)>> {
        let mut centers = BTreeMap::new();

        let sorted = (0..heatmap[0].len())
            .map(|col| heatmap.iter().map(|row| row[col]).sum::<f64>())
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
    fn cumulate_volume_profile(
        candles: &[OHCL],
        number_of_levels: usize,
        overlap: usize,
        interval_in_hour: i32,
    ) -> Result<(Vec<Vec<f64>>, Vec<f64>)> {
        if candles.is_empty() || number_of_levels == 0 {
            return Err(anyhow!(format!("candles or number_of_levels is empty")));
        }

        let mut all_volumes: Vec<Vec<f64>> = Vec::new();
        let mut price_levels: Vec<f64> = Vec::new();

        let global_min_price = candles.iter().map(|c| c.l).fold(f64::INFINITY, f64::min);
        let global_max_price = candles
            .iter()
            .map(|c| c.h)
            .fold(f64::NEG_INFINITY, f64::max);

        if global_min_price == f64::INFINITY || global_max_price == f64::NEG_INFINITY {
            return Err(anyhow!(format!("data range is out of scope")));
        }

        let price_range = global_max_price - global_min_price;
        let price_bin_size = price_range / number_of_levels as f64;

        for i in 0..number_of_levels {
            price_levels.push(global_min_price + i as f64 * price_bin_size);
        }

        if overlap == 0 {
            let mut temp_profile: HashMap<usize, f64> = HashMap::new();
            let mut volumes_for_all = vec![0.0; number_of_levels];

            for candle in candles {
                let low_bin = ((candle.l - global_min_price) / price_bin_size).floor() as usize;
                let high_bin = ((candle.h - global_min_price) / price_bin_size).floor() as usize;
                let low_bin = low_bin.min(number_of_levels - 1);
                let high_bin = high_bin.min(number_of_levels - 1);
                let num_bins = (high_bin - low_bin + 1) as f64;
                let volume_per_bin = if num_bins > 0.0 {
                    candle.v / num_bins
                } else {
                    candle.v
                };

                for bin in low_bin..=high_bin {
                    *temp_profile.entry(bin).or_insert(0.0) += volume_per_bin;
                }
            }

            for (bin_index, volume) in temp_profile {
                if bin_index < number_of_levels {
                    volumes_for_all[bin_index] = volume;
                }
            }

            return Ok((vec![volumes_for_all], price_levels));
        }

        let mut pin_start_indices: Vec<usize> = Vec::new();
        let mut current_pin = Self::timestamp_to_pin(candles[0].t, interval_in_hour);

        pin_start_indices.push(0);

        for (i, candle) in candles.iter().enumerate().skip(1) {
            let candle_pin = Self::timestamp_to_pin(candle.t, interval_in_hour);
            if candle_pin > current_pin {
                pin_start_indices.push(i);
                current_pin = candle_pin;
            }
        }

        if pin_start_indices.len() < overlap {
            return Err(anyhow!(format!("overlap is too large")));
        }

        for window_start in 0..=(pin_start_indices.len() - overlap) {
            let mut temp_profile: HashMap<usize, f64> = HashMap::new();
            let start_index = pin_start_indices[window_start];
            let end_index = if window_start + overlap < pin_start_indices.len() {
                pin_start_indices[window_start + overlap]
            } else {
                candles.len()
            };
            let window = &candles[start_index..end_index];

            for candle in window {
                let low_bin = ((candle.l - global_min_price) / price_bin_size).floor() as usize;
                let high_bin = ((candle.h - global_min_price) / price_bin_size).floor() as usize;

                let low_bin = low_bin.min(number_of_levels - 1);
                let high_bin = high_bin.min(number_of_levels - 1);

                let num_bins = (high_bin - low_bin + 1) as f64;
                let volume_per_bin = if num_bins > 0.0 {
                    candle.v / num_bins
                } else {
                    candle.v
                };

                for bin in low_bin..=high_bin {
                    *temp_profile.entry(bin).or_insert(0.0) += volume_per_bin;
                }
            }

            let mut volumes_for_window = vec![0.0; number_of_levels];
            for (bin_index, volume) in temp_profile {
                if bin_index < number_of_levels {
                    volumes_for_window[bin_index] = volume;
                }
            }

            all_volumes.push(volumes_for_window);
        }

        Ok((all_volumes, price_levels))
    }
}

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

        // Calculate with volume profile
        let mut vp1_1 = VolumeProfile::new();
        vp1_1.calculate(&candles, 10, 0, 24).unwrap();

        // Another way to calculate volume profile
        let vp2_1 = VolumeProfile::new_from_candles(&candles, 10, 0, 24).unwrap();

        // Test với overlap_days = 0 (tính toàn bộ dataset)
        let (volumes_all, levels_all) =
            VolumeProfile::cumulate_volume_profile(&candles, 10, 0, 24).unwrap();
        println!("All data volume profile: {:?}", volumes_all);
        assert_eq!(volumes_all.len(), 1); // Chỉ có 1 profile cho toàn bộ data
        assert_eq!(volumes_all.len(), vp1_1.heatmap().len());
        assert_eq!(volumes_all.len(), vp2_1.heatmap().len());

        // Test với overlap_days = 2 (sliding window theo ngày)
        let (volumes, levels) =
            VolumeProfile::cumulate_volume_profile(&candles, 10, 2, 24).unwrap();

        // Calculate with volume profile
        let mut vp1_2 = VolumeProfile::new();
        vp1_2.calculate(&candles, 10, 2, 24).unwrap();

        // Another way to calculate volume profile
        let vp2_2 = VolumeProfile::new_from_candles(&candles, 10, 2, 24).unwrap();

        println!("Number of windows: {}", volumes.len());
        println!("Price levels: {:?}", levels);
        for (i, volume_profile) in volumes.iter().enumerate() {
            println!("Window {} (2 days): {:?}", i, volume_profile);
        }

        assert_eq!(volumes.len(), 2); // Có 2 windows: ngày [1,2] và ngày [2,3]
        assert_eq!(volumes.len(), vp1_2.heatmap().len());
        assert_eq!(volumes.len(), vp2_2.heatmap().len());
    }
}
