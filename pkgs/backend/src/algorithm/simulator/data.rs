use anyhow::{anyhow, Result};
use rand::Rng;
use std::sync::Arc;

use crate::schemas::CandleStick;

#[derive(Clone, Copy)]
pub enum Phase {
    Test,
    Train,
}

#[derive(Clone)]
pub struct Data {
    candles: Arc<Vec<CandleStick>>,
    range: usize,
    begin: usize,
    split: usize,
    end: usize,
}

impl Data {
    pub fn new(candles: Arc<Vec<CandleStick>>, range: usize) -> Self {
        Self {
            candles,
            range,
            begin: 0,
            split: 0,
            end: 0,
        }
    }

    pub fn shuttle(&mut self) -> Result<()> {
        let mut rng = rand::thread_rng();
        let len = self.candles.len();
        let min_required = (self.range) * 2;
        if len < min_required {
            return Err(anyhow!(format!(
                "provided data have {} candles, but we require {} candles",
                len, min_required,
            )));
        }
        let end = rng.gen_range(min_required..=len);
        let max_begin = end.saturating_sub(min_required);
        let begin = rng.gen_range(0..=max_begin);
        let min_split = begin.saturating_add(self.range);
        let max_split = end.saturating_sub(self.range);
        let split = if min_split <= max_split {
            rng.gen_range(min_split..=max_split)
        } else {
            begin
        };
        self.begin = begin;
        self.split = split;
        self.end = end;
        Ok(())
    }

    pub fn window(&self) -> usize {
        self.range
    }

    pub fn size(&self, phase: &Phase) -> usize {
        match phase {
            Phase::Train => self
                .split
                .saturating_sub(self.begin)
                .saturating_sub(self.range)
                .saturating_add(1),
            Phase::Test => self
                .end
                .saturating_sub(self.split)
                .saturating_sub(self.range)
                .saturating_add(1),
        }
    }

    pub fn sample(&self, i: usize, phase: &Phase) -> Result<&[CandleStick]> {
        let start = match phase {
            Phase::Train => self.begin.saturating_add(i),
            Phase::Test => self.split.saturating_add(i),
        };
        let end_slice = start.saturating_add(self.range);

        if self.size(phase) > i {
            match phase {
                Phase::Train => {
                    if end_slice <= self.candles.len() {
                        Ok(&self.candles[start..end_slice])
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
                Phase::Test => Ok(if end_slice <= self.candles.len() {
                    &self.candles[start..end_slice]
                } else {
                    &self.candles[(self.candles.len() - self.range)..self.candles.len()]
                }),
            }
        } else {
            Err(anyhow!("out of range"))
        }
    }

    pub fn last_candle(&self, i: usize, phase: &Phase) -> Result<CandleStick> {
        let idx = match phase {
            Phase::Train => self.begin,
            Phase::Test => self.split,
        }
        .saturating_add(i)
        .saturating_add(self.range);

        if self.size(phase) > i {
            match phase {
                Phase::Train => {
                    if idx < self.candles.len() {
                        Ok(self.candles[idx].clone())
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
                Phase::Test => {
                    if idx < self.candles.len() {
                        Ok(self.candles[idx].clone())
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
            }
        } else {
            Err(anyhow!("out of range"))
        }
    }
}
