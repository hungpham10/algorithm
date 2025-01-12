use lib::actors::dnse::{connect_to_dnse, GetOHCLCommand};
use lib::algorithm::kmean::{LineStrategy, KMean};
use lib::schemas::CandleStick as OHCL;
use chrono::{Utc, NaiveDateTime};

use plotters::prelude::*;
use plotters::style::colors::TRANSPARENT;

fn plot(
    candles: &[OHCL], 
    ema: &[f64], 
    obv: &[f64], 
    open_trendlines: Vec<Vec<f64>>, 
    close_trendlines: Vec<Vec<f64>>, 
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (1800, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_price = candles.iter().map(|c| c.h).fold(f64::MIN, f64::max);
    let min_price = candles.iter().map(|c| c.l).fold(f64::MAX, f64::min);
    let max_ema = ema.iter().cloned().fold(f64::MIN, f64::max);
    let min_ema = ema.iter().cloned().fold(f64::MAX, f64::min);
    let max_obv = obv.iter().cloned().fold(f64::MIN, f64::max);
    let min_obv = obv.iter().cloned().fold(f64::MAX, f64::min);

    let (upper, lower) = root.split_vertically(600);

    let mut chart = ChartBuilder::on(&upper)
        .caption("Candlestick Chart", ("sans-serif", 50).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(            
            (candles.first().unwrap().t/86400)..(candles.last().unwrap().t/86400),
             min_price..max_price
        )?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        ema.iter().enumerate().map(|(i, &v)| (candles[i].t/86400, v)),
        &BLUE,
    ))?;

    chart.draw_series(candles.iter().map(|candle| {
            CandleStick::new(
                candle.t/86400,
                candle.o,
                candle.h,
                candle.l,
                candle.c,
                BLUE.filled(),
                RED.filled(),
                5,
            )
    }))?;

    for trendline in open_trendlines {
        chart.draw_series(LineSeries::new(
            obv.iter()
                .enumerate()
                .map(|(i, &v)| 
                (
                    candles[i].t/86400, 
                    trendline[0] * ((candles[i].t/86400) as f64) + trendline[1]
                )
            ),
            &BLUE,
        ))?;
    }

    for trendline in close_trendlines {
        chart.draw_series(LineSeries::new(
            obv.iter()
                .enumerate()
                .map(|(i, &v)| 
                    (
                        candles[i].t/86400, 
                        trendline[0] * ((candles[i].t/86400) as f64) + trendline[1]
                    )
                ),
            &GREEN,
        ))?;
    }

    let mut obv_chart = ChartBuilder::on(&lower)
        .caption("OBV", ("sans-serif", 50).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(            
            (candles.first().unwrap().t/86400)..(candles.last().unwrap().t/86400),
            min_obv..max_obv,
        )?;

    obv_chart.configure_mesh().draw()?;

    obv_chart.draw_series(LineSeries::new(
        obv.iter().enumerate().map(|(i, &v)| (candles[i].t/86400, v)),
        &BLUE,
    ))?;

    root.present()?;
    Ok(())
}

fn find_reverse_points(candles: &[OHCL]) -> (Vec<(i32, f64)>, Vec<(i32, f64)>) {
    let mut reverse_open_points = Vec::new();
    let mut reverse_close_points = Vec::new();
    for i in 1..candles.len() - 1 {
        if (candles[i].o > candles[i - 1].o && candles[i].o > candles[i + 1].o) ||
           (candles[i].o < candles[i - 1].o && candles[i].o < candles[i + 1].o) {
            reverse_open_points.push((candles[i].t, candles[i].o));
        }
        if (candles[i].c > candles[i - 1].c && candles[i].c > candles[i + 1].c) ||
           (candles[i].c < candles[i - 1].c && candles[i].c < candles[i + 1].c) {
            reverse_close_points.push((candles[i].t, candles[i].c));
        }
    }

    (reverse_open_points, reverse_close_points)
}

fn calculate_obv(candles: &[OHCL]) -> Vec<f64> {
    let mut obv = Vec::with_capacity(candles.len());
    let mut current_obv = 0.0;

    for i in 1..candles.len() {
        if candles[i].c > candles[i - 1].c {
            current_obv += candles[i].v as f64;
        } else if candles[i].c < candles[i - 1].c {
            current_obv -= candles[i].v as f64;
        }
        obv.push(current_obv);
    }

    obv
}

fn calculate_ema(candles: &[OHCL], period: usize) -> Vec<f64> {
    let mut ema = Vec::with_capacity(candles.len());
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut prev_ema = candles[0].c;

    for candle in candles.iter() {
        let current_ema = (candle.c - prev_ema) * multiplier + prev_ema;
        ema.push(current_ema);
        prev_ema = current_ema;
    }

    ema
}

#[actix_rt::test]
async fn test_obv() {
    let dnse = connect_to_dnse();
    let symbol = "MWG";
    let candles_1d = dnse.send(GetOHCLCommand{
        resolution: String::from("1D"),
        stock: String::from(symbol),
        from: Utc::now().timestamp() - 1800*24*60*60,
        to: Utc::now().timestamp() - 400*24*60*60,
    })
    .await
    .unwrap()
    .unwrap();

    let obv_1d = calculate_obv(&candles_1d);
    let ema55_1d = calculate_ema(&candles_1d, 55);

    let (reverse_open_points, reverse_close_points) = find_reverse_points(&candles_1d);
    let k = 10;

    let mut open_kmean = KMean::new(
        k,  // Number of clusters
        2000, // Max iterations
        LineStrategy::new(k, (-10.0, 10.0), (-10.0, 10.0)),
    );
    let mut close_kmean = KMean::new(
        k,  // Number of clusters
        2000, // Max iterations
        LineStrategy::new(k, (-10.0, 10.0), (-10.0, 10.0)),
    );

    open_kmean.insert(&reverse_open_points
        .iter()
        .map(|x| ((x.0/86400) as f64, x.1))
        .collect::<Vec<(f64, f64)>>(),
    );
    open_kmean.commit();

    close_kmean.insert(&reverse_close_points
        .iter()
        .map(|x| ((x.0/86400) as f64, x.1))
        .collect::<Vec<(f64, f64)>>(),
    );
    close_kmean.commit();

    println!("{:?}", open_kmean.fit());
    println!("{:?}", close_kmean.fit());

    for i in 0..k {
        println!("{:?}: {:?} {:?}", i, open_kmean.cluster(i), close_kmean.cluster(i));
        open_kmean.points(i)
            .iter()
            .for_each(|x| {
                println!("{:?} {:?}", NaiveDateTime::from_timestamp((x.0 as i64) * 86400, 0), x.1);
            });
    }

    plot(
        &candles_1d, 
        &ema55_1d, 
        &obv_1d, 
        ((0)..k).map(|i| open_kmean.cluster(i) )
            .collect::<Vec<Vec<f64>>>(),
        ((0)..k).map(|i| close_kmean.cluster(i) )
            .collect::<Vec<Vec<f64>>>(),
        "obv_1d.png",
    ).unwrap();
}

