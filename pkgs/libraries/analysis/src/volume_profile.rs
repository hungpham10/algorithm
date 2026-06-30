use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};

use itertools::Itertools;
use rayon::prelude::*;
use schemas::CandleStick;

#[cfg(target_arch = "x86_64")]
mod simd {
    use std::arch::x86_64::*;

    #[target_feature(enable = "avx")]
    pub unsafe fn sum_f64(slice: &[f64]) -> f64 {
        let mut acc = _mm256_setzero_pd();
        let mut i = 0;
        while i + 4 <= slice.len() {
            // Wrap every intrinsic in unsafe {}
            let vals = unsafe { _mm256_loadu_pd(slice.as_ptr().add(i)) };
            acc = _mm256_add_pd(acc, vals);
            i += 4;
        }
        let mut buf = [0f64; 4];
        unsafe { _mm256_storeu_pd(buf.as_mut_ptr(), acc) };

        let mut total = buf.iter().sum::<f64>();
        // Clean remaining elements
        total += slice[i..].iter().sum::<f64>();
        total
    }

    #[target_feature(enable = "avx")]
    pub unsafe fn add_scalar(volumes: &mut [f64], low_bin: usize, high_bin: usize, val: f64) {
        let vec_val = _mm256_set1_pd(val);
        let mut i = low_bin;
        // Process in chunks of 4
        while i + 4 <= high_bin + 1 {
            let ptr = unsafe { volumes.as_mut_ptr().add(i) };
            unsafe {
                let old = _mm256_loadu_pd(ptr);
                let new = _mm256_add_pd(old, vec_val);
                _mm256_storeu_pd(ptr, new);
            }
            i += 4;
        }
        // FIX: needless_range_loop
        for v in volumes.iter_mut().take(high_bin + 1).skip(i) {
            *v += val;
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod simd {
    use std::arch::aarch64::*;

    #[target_feature(enable = "neon")]
    #[inline]
    pub unsafe fn sum_f64(slice: &[f64]) -> f64 {
        let mut acc = vdupq_n_f64(0.0);
        let mut i = 0;
        while i + 2 <= slice.len() {
            let vals = unsafe { vld1q_f64(slice[i..].as_ptr()) };
            acc = vaddq_f64(acc, vals);
            i += 2;
        }
        let mut buf = [0f64; 2];

        unsafe {
            vst1q_f64(buf.as_mut_ptr(), acc);
        }

        let mut total = buf.iter().sum::<f64>();
        for &v in &slice[i..] {
            total += v;
        }
        total
    }

    #[target_feature(enable = "neon")]
    #[inline]
    pub unsafe fn add_scalar(volumes: &mut [f64], low_bin: usize, high_bin: usize, val: f64) {
        let vec_val = vdupq_n_f64(val);
        let mut i = low_bin;
        while i + 2 <= high_bin + 1 {
            let ptr = volumes[i..].as_mut_ptr();
            let old = unsafe { vld1q_f64(ptr) };
            let new = vaddq_f64(old, vec_val);

            unsafe {
                vst1q_f64(ptr, new);
            }

            i += 2;
        }

        volumes[i..=high_bin].iter_mut().for_each(|v| *v += val);
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod simd {
    #[inline]
    pub fn sum_f64(slice: &[f64]) -> f64 {
        slice.iter().sum()
    }
}

pub struct VolumeProfile {
    heatmap: Vec<Vec<f64>>,
    levels: Vec<f64>,
    ranges: Vec<(usize, usize, usize)>,
    /// Mỗi phần tử là segments (start, end) của một range —
    /// thời điểm volume bắt đầu xuất hiện & tích luỹ trong vùng giá đó.
    timelines: Vec<Vec<(usize, usize)>>,
}
type VolumeRange = (usize, usize);
type VolumeTimeline = Vec<VolumeRange>;

impl Default for VolumeProfile {
    fn default() -> Self {
        Self::new()
    }
}

impl VolumeProfile {
    pub fn new() -> Self {
        Self {
            heatmap: Vec::new(),
            levels: Vec::new(),
            ranges: Vec::new(),
            timelines: Vec::new(),
        }
    }

    pub fn new_from_candles(
        candles: &[CandleStick],
        number_of_levels: usize,
        overlap: usize,
        interval_in_hour: i32,
    ) -> Result<Self, Error> {
        let (heatmap, levels) =
            Self::cumulate_volume_profile(candles, number_of_levels, overlap, interval_in_hour)?;
        let ranges = Self::cumulate_volume_range(&heatmap, number_of_levels / 10)?;
        let timelines = Self::calculate_cumulate_volume_timeline(&heatmap, &ranges)?;

        Ok(Self {
            heatmap,
            levels,
            ranges,
            timelines,
        })
    }

    pub fn calculate(
        &mut self,
        candles: &[CandleStick],
        number_of_levels: usize,
        overlap: usize,
        interval_in_hour: i32,
    ) -> Result<(), Error> {
        let (heatmap, levels) =
            Self::cumulate_volume_profile(candles, number_of_levels, overlap, interval_in_hour)?;

        self.ranges = Self::cumulate_volume_range(&heatmap, number_of_levels / 10)?;
        self.timelines = Self::calculate_cumulate_volume_timeline(&heatmap, &self.ranges)?;
        self.levels = levels;
        self.heatmap = heatmap;
        Ok(())
    }

    // --- Getters ---
    pub fn heatmap(&self) -> &Vec<Vec<f64>> {
        &self.heatmap
    }
    pub fn levels(&self) -> &Vec<f64> {
        &self.levels
    }
    pub fn ranges(&self) -> &Vec<(usize, usize, usize)> {
        &self.ranges
    }
    pub fn timelines(&self) -> &Vec<Vec<(usize, usize)>> {
        &self.timelines
    }

    #[inline]
    fn timestamp_to_pin(t: i32, interval_in_hour: i32) -> i32 {
        t / (interval_in_hour * 60 * 60)
    }

    /// Tính timeline cho mỗi range: tìm các khoảng thời gian (row index) liên tục
    /// mà **tổng volume** của tất cả cột (price level) trong range vượt ngưỡng.
    ///
    /// Ngưỡng = 1% của tổng volume tối đa một row trong range đó,
    /// giúp loại bỏ nhiễu và chỉ giữ lại các khoảng tích luỹ thực sự.
    /// Các segment chỉ có 1 row (isolated) bị lọc bỏ vì "tích luỹ" cần >= 2 row liên tục.
    ///
    /// Trả về `Vec<Vec<(usize, usize)>>` — mỗi range một danh sách các segment (start, end).
    pub fn calculate_cumulate_volume_timeline(
        heatmap: &[Vec<f64>],
        ranges: &[(usize, usize, usize)],
    ) -> Result<Vec<VolumeTimeline>, Error> {
        if heatmap.is_empty() || ranges.is_empty() {
            return Ok(Vec::new());
        }

        Ok(ranges
            .par_iter()
            .map(|(_, l_beg, l_end)| {
                let cols: Vec<usize> = (*l_beg..=*l_end).collect();
                let row_count = heatmap.len();

                // Tính tổng volume mỗi row trong range
                let row_totals: Vec<f64> = (0..row_count)
                    .into_par_iter()
                    .map(|row| cols.iter().map(|&col| heatmap[row][col]).sum())
                    .collect();

                // Ngưỡng: 1% của max row total, nhưng tối thiểu > 0 để tránh nhiễu
                let max_total = row_totals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let threshold = if max_total > 0.0 {
                    (max_total * 0.01).max(f64::EPSILON)
                } else {
                    f64::EPSILON
                };

                // Xác định các row có volume tích luỹ > ngưỡng
                let active_rows: Vec<bool> = (0..row_count)
                    .into_par_iter()
                    .map(|row| row_totals[row] > threshold)
                    .collect();

                // Gom các row liên tục thành segments, lọc bỏ segment đơn (cần >= 2 row để gọi là tích luỹ)
                let mut segments = Vec::new();
                let mut i = 0;
                while i < row_count {
                    if active_rows[i] {
                        let seg_start = i;
                        while i < row_count && active_rows[i] {
                            i += 1;
                        }
                        // Chỉ giữ segment có ít nhất 2 row (tích luỹ thực sự)
                        if i - seg_start >= 2 {
                            segments.push((seg_start, i));
                        }
                    } else {
                        i += 1;
                    }
                }
                segments
            })
            .collect())
    }

    #[inline]
    fn smooth_column_totals(totals: &[f64], window: usize) -> Vec<f64> {
        let half = window / 2;
        (0..totals.len())
            .map(|i| {
                let start = i.saturating_sub(half);
                let end = usize::min(i + half + 1, totals.len());
                (unsafe { simd::sum_f64(&totals[start..end]) }) / (end - start) as f64
            })
            .collect()
    }

    #[inline]
    pub fn cumulate_volume_range(
        heatmap: &[Vec<f64>],
        window: usize,
    ) -> Result<Vec<(usize, usize, usize)>, Error> {
        if heatmap.is_empty() || heatmap[0].is_empty() {
            return Ok(Vec::new());
        }

        // Song song hóa + SIMD: tính tổng volume của từng cột
        let column_totals: Vec<f64> = (0..heatmap[0].len())
            .into_par_iter()
            .map(|col| {
                // dùng SIMD sum thay vì .sum::<f64>()
                let col_slice: Vec<f64> = heatmap.iter().map(|row| row[col]).collect();
                unsafe { simd::sum_f64(&col_slice) }
            })
            .collect();

        // Làm mượt bằng SIMD
        let smoothed = Self::smooth_column_totals(&column_totals, window);

        // Sắp xếp theo volume giảm dần
        let sorted = smoothed
            .iter()
            .enumerate()
            .sorted_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(Ordering::Greater))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        // Gom nhóm các cột thành range
        let mut centers = BTreeMap::new();
        for t in sorted {
            let mut found = false;
            let mut update_data = None;

            for (group, (begin, end, order)) in &centers {
                if t + 1 == *begin {
                    update_data = Some((*group, t, *end, *order));
                    found = true;
                    break;
                }
                if t > 0 && t - 1 == *end {
                    update_data = Some((*group, *begin, t, *order));
                    found = true;
                    break;
                }
            }

            if let Some((group, new_beg, new_end, order)) = update_data {
                centers.insert(group, (new_beg, new_end, order));
            } else if !found && !centers.contains_key(&t) {
                centers.insert(t, (t, t, centers.len()));
            }
        }

        Ok(centers
            .into_iter()
            .sorted_by_key(|k| k.1.2)
            .map(|(center, (begin, end, _))| (begin, center, end))
            .collect())
    }

    #[inline]
    fn cumulate_volume_profile(
        candles: &[CandleStick],
        number_of_levels: usize,
        overlap: usize,
        interval_in_hour: i32,
    ) -> Result<(Vec<Vec<f64>>, Vec<f64>), Error> {
        if candles.is_empty() || number_of_levels == 0 {
            return Err(Error::new(ErrorKind::InvalidData, "Empty data"));
        }

        let global_min_price = candles.iter().map(|c| c.l).fold(f64::INFINITY, f64::min);
        let global_max_price = candles
            .iter()
            .map(|c| c.h)
            .fold(f64::NEG_INFINITY, f64::max);

        let price_range = global_max_price - global_min_price;
        if price_range <= 0.0 {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid price range"));
        }

        let price_bin_size = price_range / number_of_levels as f64;
        let price_levels: Vec<f64> = (0..number_of_levels)
            .map(|i| global_min_price + i as f64 * price_bin_size)
            .collect();

        // Chia pin theo interval
        let mut pin_start_indices = vec![0];
        let mut current_pin = Self::timestamp_to_pin(candles[0].t, interval_in_hour);

        for (i, candle) in candles.iter().enumerate().skip(1) {
            let candle_pin = Self::timestamp_to_pin(candle.t, interval_in_hour);
            if candle_pin > current_pin {
                pin_start_indices.push(i);
                current_pin = candle_pin;
            }
        }

        // Trường hợp overlap = 0 (tính toàn bộ dataset)
        if overlap == 0 {
            let mut volumes = vec![0.0; number_of_levels];
            for candle in candles {
                Self::fill_volumes(
                    &mut volumes,
                    candle,
                    global_min_price,
                    price_bin_size,
                    number_of_levels,
                );
            }
            return Ok((vec![volumes], price_levels));
        }

        if pin_start_indices.len() < overlap {
            return Err(Error::new(ErrorKind::InvalidData, "Overlap is too large"));
        }

        // Song song hóa + SIMD bên trong
        let heatmap: Vec<Vec<f64>> = (0..=(pin_start_indices.len() - overlap))
            .into_par_iter()
            .map(|window_start| {
                let mut window_volumes = vec![0.0; number_of_levels];
                let start_idx = pin_start_indices[window_start];
                let end_idx = pin_start_indices
                    .get(window_start + overlap)
                    .cloned()
                    .unwrap_or(candles.len());

                for candle in &candles[start_idx..end_idx] {
                    // gọi phiên bản SIMD
                    Self::fill_volumes(
                        &mut window_volumes,
                        candle,
                        global_min_price,
                        price_bin_size,
                        number_of_levels,
                    );
                }
                window_volumes
            })
            .collect();

        Ok((heatmap, price_levels))
    }

    #[inline]
    fn fill_volumes(
        volumes: &mut [f64],
        candle: &CandleStick,
        min_p: f64,
        bin_size: f64,
        levels: usize,
    ) {
        let low_bin = ((candle.l - min_p) / bin_size).floor() as usize;
        let high_bin = ((candle.h - min_p) / bin_size).floor() as usize;
        let low_bin = low_bin.min(levels - 1);
        let high_bin = high_bin.min(levels - 1);

        let num_bins = (high_bin - low_bin + 1) as f64;
        let vol_per_bin = candle.v / num_bins;

        unsafe {
            simd::add_scalar(volumes, low_bin, high_bin, vol_per_bin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cumulate_volume_profile() {
        let candles = vec![
            // Ngày 1 (t: 1000000000 = 2001-09-09)
            CandleStick {
                o: 100.0,
                h: 105.0,
                c: 102.0,
                l: 98.0,
                v: 1000.0,
                t: 1000000000,
            },
            CandleStick {
                o: 102.0,
                h: 108.0,
                c: 106.0,
                l: 101.0,
                v: 1500.0,
                t: 1000010000,
            },
            // Ngày 2 (t: 1000086400 = 2001-09-10)
            CandleStick {
                o: 106.0,
                h: 110.0,
                c: 109.0,
                l: 104.0,
                v: 2000.0,
                t: 1000086400,
            },
            CandleStick {
                o: 109.0,
                h: 112.0,
                c: 111.0,
                l: 107.0,
                v: 1200.0,
                t: 1000096400,
            },
            // Ngày 3 (t: 1000172800 = 2001-09-11)
            CandleStick {
                o: 111.0,
                h: 115.0,
                c: 113.0,
                l: 109.0,
                v: 1800.0,
                t: 1000172800,
            },
            CandleStick {
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
        let (volumes_all, _levels_all) =
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
