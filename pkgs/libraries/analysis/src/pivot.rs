use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind};

use ordered_float::OrderedFloat;
use schemas::CandleStick;

struct Pivot {
    metrics: Vec<Vec<usize>>,
    range: (usize, usize),
}

/*
    l = count(x -> x > df.c[i], @view df.c[i-w:i-1])
    r = count(x -> x > df.c[i], @view df.c[i+1:i+w])
    peak = 1 - (w - l) * (w - r) / w^2
    peak = 1 - (w - l) / w

    l = count(x -> x > df.c[i], @view df.w[i-w:i-1])
    r = count(x -> x > df.c[i], @view df.w[i+1:i+w])
    degree = 1 - (w - l) * (w - r) / w^2
    degree = 1 - (w - l) / w

    spread = (df.c[i] - df.o[i]) / (df.h[i] - df.l[i])
    vol cạn kiệt dần

    lim(degree) ~= 1
    lim(abs(spread)) ~= 1
*/

#[derive(Clone)]
struct FenwickTree {
    tree: Vec<usize>,
}

impl FenwickTree {
    fn new(n: usize) -> Self {
        Self {
            tree: vec![0; n + 1],
        }
    }
    fn add(&mut self, mut i: usize, delta: isize) {
        i += 1;
        while i < self.tree.len() {
            if delta > 0 {
                self.tree[i] += 1;
            } else {
                self.tree[i] -= 1;
            }
            i += i & (!i + 1);
        }
    }
    fn query(&self, mut i: usize) -> usize {
        i += 1;
        let mut sum = 0;
        while i > 0 {
            sum += self.tree[i];
            i -= i & (!i + 1);
        }
        sum
    }
}

impl Pivot {
    pub fn new(candles: &[CandleStick]) -> Result<Self, Error> {
        let range = (0, candles.len() / 3);
        let metrics = Self::cumulate_pivot_in_multiple_windows(candles, range.0, range.1)?;

        Ok(Self { metrics, range })
    }

    #[inline]
    fn cumulate_pivot_in_multiple_windows(
        candles: &[CandleStick],
        min_window_size: usize,
        max_window_size: usize,
    ) -> Result<Vec<Vec<usize>>, Error> {
        let n = candles.len();

        if max_window_size > n / 2 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "max_window_size too large",
            ));
        }

        let num_windows = max_window_size - min_window_size + 1;

        let mut sorted_prices = candles
            .iter()
            .map(|c| OrderedFloat(c.c))
            .collect::<Vec<_>>();

        sorted_prices.sort_unstable();
        sorted_prices.dedup();

        let rank_map = sorted_prices
            .iter()
            .enumerate()
            .map(|(idx, price)| (price, idx))
            .collect::<HashMap<_, _>>();

        let mut fts = vec![FenwickTree::new(rank_map.len()); num_windows];
        let mut streams = vec![VecDeque::with_capacity(max_window_size); num_windows];
        let mut results = Vec::with_capacity(n);

        for candle in candles {
            let current_price = OrderedFloat(candle.c);
            let current_rank = *rank_map.get(&current_price).unwrap();
            let mut current_candle_results = Vec::with_capacity(num_windows);

            for w_idx in 0..num_windows {
                let current_w_limit = min_window_size + w_idx;
                let ft = &mut fts[w_idx];
                let stream = &mut streams[w_idx];

                ft.add(current_rank, 1);
                stream.push_back(candle.c);

                if stream.len() > current_w_limit
                    && let Some(old_val) = stream.pop_front()
                {
                    ft.add(*rank_map.get(&OrderedFloat(old_val)).unwrap(), -1);
                }

                let total_in_window = stream.len();
                let count_smaller_equal = ft.query(current_rank);
                current_candle_results.push(total_in_window - count_smaller_equal);
            }

            results.push(current_candle_results);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pivot_logic_simple() {
        // Tạo dữ liệu giả: giá tăng dần
        let candles = vec![
            CandleStick {
                c: 10.0,
                ..Default::default()
            },
            CandleStick {
                c: 20.0,
                ..Default::default()
            },
            CandleStick {
                c: 30.0,
                ..Default::default()
            },
            CandleStick {
                c: 40.0,
                ..Default::default()
            },
            CandleStick {
                c: 50.0,
                ..Default::default()
            },
        ];

        // Thử nghiệm với window size = 2
        // min = 2, max = 2 -> chỉ có 1 window duy nhất
        let result = Pivot::cumulate_pivot_in_multiple_windows(&candles, 2, 2).unwrap();

        assert_eq!(result.len(), 5);

        // Giải thích logic mong đợi cho window size 2:
        // i=0: [10], count(>10) = 0
        // i=1: [10, 20], count(>20) = 0
        // i=2: [20, 30], count(>30) = 0
        for res in result {
            assert_eq!(
                res[0], 0,
                "Vì giá tăng dần nên không có nến nào lớn hơn nến hiện tại"
            );
        }
    }

    #[test]
    fn test_pivot_logic_with_peaks() {
        // Giá tạo đỉnh ở giữa: [10, 50, 20]
        let candles = vec![
            CandleStick {
                c: 10.0,
                ..Default::default()
            },
            CandleStick {
                c: 50.0,
                ..Default::default()
            },
            CandleStick {
                c: 20.0,
                ..Default::default()
            },
            CandleStick {
                c: 15.0,
                ..Default::default()
            },
        ];

        let result = Pivot::cumulate_pivot_in_multiple_windows(&candles, 3, 3).unwrap();

        // Xét tại nến cuối cùng (i=3, giá 15.0):
        // Window size 3 bao gồm các nến i=1, 2, 3: [50.0, 20.0, 15.0]
        // Số nến lớn hơn 15.0 là: 50.0 và 20.0 -> Kết quả phải là 2
        assert_eq!(result[3][0], 2);
    }

    #[test]
    fn test_multiple_windows() {
        let candles = vec![
            CandleStick {
                c: 10.0,
                ..Default::default()
            },
            CandleStick {
                c: 40.0,
                ..Default::default()
            },
            CandleStick {
                c: 20.0,
                ..Default::default()
            },
            CandleStick {
                c: 30.0,
                ..Default::default()
            },
        ];

        // Test đồng thời window 2 và 3
        let result = Pivot::cumulate_pivot_in_multiple_windows(&candles, 2, 3).unwrap();

        // Tại nến cuối (i=3, giá 30.0):
        // Window 2: [20.0, 30.0] -> count(>30) = 0
        // Window 3: [40.0, 20.0, 30.0] -> count(>30) = 1 (là nến 40.0)
        assert_eq!(result[3][0], 0); // window index 0 (size 2)
        assert_eq!(result[3][1], 1); // window index 1 (size 3)
    }

    #[test]
    fn test_invalid_input() {
        let candles = vec![CandleStick {
            c: 10.0,
            ..Default::default()
        }];
        // max_window_size (10) > candles.len()/2 (0) -> Phải trả về Error
        let result = Pivot::cumulate_pivot_in_multiple_windows(&candles, 2, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_prices() {
        let candles = vec![
            CandleStick {
                c: 20.0,
                ..Default::default()
            },
            CandleStick {
                c: 20.0,
                ..Default::default()
            },
            CandleStick {
                c: 20.0,
                ..Default::default()
            },
        ];
        let result = Pivot::cumulate_pivot_in_multiple_windows(&candles, 2, 2).unwrap();

        // Vì giá bằng nhau, count(x > current) luôn là 0
        for res in result {
            assert_eq!(res[0], 0);
        }
    }
}
