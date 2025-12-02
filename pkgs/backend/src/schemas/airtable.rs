use airtable_api::{Airtable, Record};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use redis::{AsyncCommands, Client as RedisClient};

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
    enabled: bool,
    ttl: u64,
    redis: Option<RedisClient>,
}

impl Portal {
    pub fn new(
        api_key: &str,
        base_id: &str,
        mapping: &HashMap<String, String>,
        redis: Option<RedisClient>,
        enabled: bool,
    ) -> Self {
        let airtable = Airtable::new(api_key, base_id, "");
        let mapping = mapping.clone();

        Self {
            airtable,
            mapping,
            enabled,
            redis,

            // @NOTE: keep at least 1 month
            ttl: 30 * 24 * 60 * 60,
        }
    }

    pub async fn watchlist(&self) -> Result<Vec<Record<WatchList>>> {
        if self.enabled {
            let watchlist_table = match self.mapping.get(&WATCHLIST.to_string()) {
                Some(table) => Ok(table),
                None => Err(anyhow::anyhow!(
                    "Please define which table will be {}",
                    WATCHLIST
                )),
            }?;

            if let Some(client) = self.redis.as_ref() {
                if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                    let cached: Option<String> =
                        conn.get(format!("airtable:{}", WATCHLIST)).await.ok();

                    if let Some(data) = cached {
                        if let Ok(records) = serde_json::from_str::<Vec<Record<WatchList>>>(&data) {
                            return Ok(records);
                        }
                    }
                }
            }

            let records = self
                .airtable
                .list_records(
                    watchlist_table.as_str(),
                    "Watch",
                    vec!["Symbol", "OrderFlow"],
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch WatchList from Airtable: {}", e))?;

            if let Some(client) = self.redis.as_ref() {
                if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                    if let Ok(json) = serde_json::to_string(&records) {
                        conn.set_ex::<_, _, ()>(format!("airtable:{}", WATCHLIST), json, self.ttl)
                            .await
                            .map_err(|error| anyhow::anyhow!("Failed to update cache {}", error))?;
                    }
                }
            }

            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn cronjob(&self) -> Result<Vec<Record<Cronjob>>> {
        if self.enabled {
            if let Some(client) = self.redis.as_ref() {
                if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                    let cached: Option<String> =
                        conn.get(format!("airtable:{}", CRONJOB)).await.ok();

                    if let Some(data) = cached {
                        if let Ok(records) = serde_json::from_str::<Vec<Record<Cronjob>>>(&data) {
                            return Ok(records);
                        }
                    }
                }
            }

            let cronjob_table = match self.mapping.get(&CRONJOB.to_string()) {
                Some(table) => Ok(table),
                None => Err(anyhow::anyhow!(
                    "Please define which table will be {}",
                    CRONJOB
                )),
            }?;

            let records = self
                .airtable
                .list_records(
                    cronjob_table.as_str(),
                    "Cron",
                    vec!["Crontime", "Route", "Timeout", "Fuzzy"],
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch Cronjobs from Airtable: {}", e))?;

            if let Some(client) = self.redis.as_ref() {
                if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                    if let Ok(json) = serde_json::to_string(&records) {
                        conn.set_ex::<_, _, ()>(format!("airtable:{}", CRONJOB), json, self.ttl)
                            .await
                            .map_err(|error| anyhow::anyhow!("Failed to update cache {}", error))?;
                    }
                }
            }

            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }
}
