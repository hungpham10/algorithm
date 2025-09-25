use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use log::info;

use vnscope::algorithm::fuzzy::replay;
use vnscope::algorithm::fuzzy::{
    Delegate, Expression, Format, Function, Pin, Rule, RuleError, Variables,
};
use vnscope::{expr, input, nested};

struct Bookmap {
    profile: RefCell<BTreeMap<i64, f64>>,
}

impl Bookmap {
    fn new() -> Self {
        Self {
            profile: RefCell::new(BTreeMap::new()),
        }
    }

    fn dump(&self) {
        self.profile.borrow().iter().for_each(|(key, val)| {
            info!("{} -> {}", (*key as f64) / 1000.0, val);
        });
    }
}

impl Function for Bookmap {
    fn evaluate(&self, _: &Rule, pins: Vec<(String, f64)>) -> Result<f64, RuleError> {
        if pins.len() < 2 {
            Err(RuleError {
                message: "Bookmap function requires at least 2 arguments".to_string(),
            })
        } else {
            let mut profile = self.profile.borrow_mut();
            let price = (pins[0].1 * 1000.0) as i64;

            if profile.contains_key(&price) {
                if profile[&price] < pins[1].1 {
                    //info!("{}: {} -> {}", price, profile[&price], pins[1].1);
                }
                if profile[&price] > pins[1].1 && pins[1].1 > 0.0 {
                    info!(
                        "{}/{}: {} -> {}",
                        price, pins[2].1, profile[&price], pins[1].1
                    );
                }
            }

            profile.insert(price, pins[1].1);
            Ok(profile.len() as f64)
        }
    }
}

#[tokio::test]
async fn test_function_map() {
    dotenvy::dotenv().ok();

    let bookmap = Arc::new(Bookmap::new());
    let rule = Delegate::new()
        .add("bookmap", bookmap.clone() as Arc<dyn Function>)
        .build(
            &(expr!(
                "chain",
                nested!(
                    "BCM.bookmap",
                    "bookmap",
                    nested!("BCM.depth", "as", input!("BCM.price_plus1[0]")),
                    nested!("BCM.volume", "negative", input!("BCM.volume_plus1[0]")),
                    input!("BCM.price[0]")
                ),
                nested!(
                    "BCM.bookmap",
                    "bookmap",
                    nested!("BCM.depth", "as", input!("BCM.price_plus2[0]")),
                    nested!("BCM.volume", "negative", input!("BCM.volume_plus2[0]")),
                    input!("BCM.price[0]")
                ),
                nested!(
                    "BCM.bookmap",
                    "bookmap",
                    nested!("BCM.depth", "as", input!("BCM.price_plus3[0]")),
                    nested!("BCM.volume", "negative", input!("BCM.volume_plus3[0]")),
                    input!("BCM.price[0]")
                ),
                nested!(
                    "BCM.bookmap",
                    "bookmap",
                    nested!("BCM.depth", "as", input!("BCM.price_minus1[0]")),
                    input!("BCM.volume_minus1[0]"),
                    input!("BCM.price[0]")
                ),
                nested!(
                    "BCM.bookmap",
                    "bookmap",
                    nested!("BCM.depth", "as", input!("BCM.price_minus2[0]")),
                    input!("BCM.volume_minus2[0]"),
                    input!("BCM.price[0]")
                ),
                nested!(
                    "BCM.bookmap",
                    "bookmap",
                    nested!("BCM.depth", "as", input!("BCM.price_minus3[0]")),
                    input!("BCM.volume_minus3[0]"),
                    input!("BCM.price[0]")
                )
            )),
            Format::Expression,
        )
        .unwrap();

    let mut vars = Variables::new_with_s3(
        100,
        1000,
        "vnscope",
        "vps",
        Some("us-or"),
        Some("https://k1x0.or1.idrivee2-72.com"),
    )
    .await;
    let symbol = "BCM".to_string();

    for timestamp in vars
        .list_from_s3(symbol.as_str(), "investing/2025-07-23")
        .await
        .unwrap()
    {
        let (mut reader, num_of_rows) =
            vars.read_from_s3(symbol.as_str(), timestamp).await.unwrap();

        replay(&mut reader, num_of_rows, &symbol, &rule)
            .await
            .unwrap();
    }
}
