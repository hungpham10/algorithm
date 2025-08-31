use airtable_api::{Airtable, Record};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct WatchList {
    #[serde(rename = "Symbol")]
    pub symbol: Option<String>,

    #[serde(rename = "OrderFlow")]
    pub use_order_flow: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cronjob {
    #[serde(rename = "Crontime")]
    pub cron: Option<String>,

    #[serde(rename = "Route")]
    pub route: Option<String>,

    #[serde(rename = "Timeout")]
    pub timeout: Option<i32>,

    #[serde(rename = "Fuzzy")]
    pub fuzzy: Option<String>,
}

pub const WATCHLIST: &str = "WatchList";
pub const CRONJOB: &str = "Cronjob";

pub struct Portal {
    airtable: Airtable,
    mapping: HashMap<String, String>,
}

impl Portal {
    pub fn new(api_key: &str, base_id: &str, mapping: &HashMap<String, String>) -> Self {
        let airtable = Airtable::new(api_key, base_id, "");
        let mapping = mapping.clone();
        Self { airtable, mapping }
    }

    pub async fn watchlist(&self) -> Result<Vec<Record<WatchList>>> {
        let watchlist_table = match self.mapping.get(&WATCHLIST.to_string()) {
            Some(table) => Ok(table),
            None => Err(anyhow::anyhow!(
                "Please define which table will be {}",
                WATCHLIST
            )),
        }?;

        self.airtable
            .list_records(
                watchlist_table.as_str(),
                "Watch",
                vec!["Symbol", "OrderFlow"],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch WatchList from Airtable: {}", e))
    }

    pub async fn cronjob(&self) -> Result<Vec<Record<Cronjob>>> {
        let cronjob_table = match self.mapping.get(&CRONJOB.to_string()) {
            Some(table) => Ok(table),
            None => Err(anyhow::anyhow!(
                "Please define which table will be {}",
                CRONJOB
            )),
        }?;

        self.airtable
            .list_records(
                cronjob_table.as_str(),
                "Cron",
                vec!["Crontime", "Route", "Timeout", "Fuzzy"],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch Cronjobs from Airtable: {}", e))
    }
}
