use lib::actors::dnse::{connect_to_dnse, GetOHCLCommand};
use lib::algorithm::math::find_reverse_points;
use chrono::Utc;

#[actix_rt::test]
async fn test_obv() {
    let dnse = connect_to_dnse();
    let symbol = "BFC";
    let candles_1d = dnse.send(GetOHCLCommand{
        resolution: String::from("1D"),
        stock: String::from(symbol),
        from: Utc::now().timestamp() - 200*24*60*60,
        to: Utc::now().timestamp(),
    })
    .await
    .unwrap()
    .unwrap();

    find_trend_lines(find_reverse_points(candles_1d.as_slice()));
}

fn find_trend_lines(
    points: Vec<(i32, f64)>
) -> Option<Vec<(f64, f64)>> {
    return None;
}
