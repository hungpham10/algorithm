use schemas::CandleStick;
use std::io::{Error, ErrorKind};

fn weighted_moving_average(data: &[f64], period: usize) -> Vec<f64> {
    if data.len() < period {
        return vec![];
    }

    let mut wma = Vec::with_capacity(data.len());
    let weights: Vec<f64> = (1..=period).map(|x| x as f64).collect();
    let weight_sum: f64 = weights.iter().sum();

    for i in (period - 1)..data.len() {
        let mut sum = 0.0;
        for j in 0..period {
            sum += data[i - j] * weights[period - 1 - j];
        }
        wma.push(sum / weight_sum);
    }
    wma
}

pub fn calculate_rrg(
    target: &[CandleStick],
    reference: &[CandleStick],
    period: usize,
) -> Result<Vec<(f64, f64)>, Error> {
    if target.is_empty() || reference.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Target or reference data is empty",
        ));
    }

    if target.len() < period * 3 {
        return Err(Error::new(ErrorKind::InvalidInput, "Data too short"));
    }

    if target.len() != reference.len() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Target and reference candle counts do not match: target={}, reference={}",
                target.len(),
                reference.len()
            ),
        ));
    }

    let rs_ratio = weighted_moving_average(
        &weighted_moving_average(
            &target
                .iter()
                .zip(reference.iter())
                .map(|(t, r)| 100.0 * t.c / r.c)
                .collect::<Vec<_>>(),
            period,
        ),
        period,
    );
    let rs_momentum = weighted_moving_average(
        &rs_ratio
            .windows(period + 1)
            .map(|w| (w[period] / w[0] - 1.0) * 100.0 + 100.0)
            .collect::<Vec<_>>(),
        period,
    );
    let offset = rs_ratio.len() - rs_momentum.len();

    Ok(rs_momentum
        .iter()
        .enumerate()
        .map(|(i, &momentum)| (rs_ratio[i + offset], momentum))
        .collect::<Vec<_>>())
}
