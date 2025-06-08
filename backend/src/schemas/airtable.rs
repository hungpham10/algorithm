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
    /// Creates a new `Portal` instance configured to access a specific Airtable base.
    ///
    /// # Examples
    ///
    /// ```
    /// let portal = Portal::new("your_api_key", "your_base_id");
    /// ```
    pub fn new(api_key: &str, base_id: &str, mapping: &HashMap<String, String>) -> Self {
        let airtable = Airtable::new(api_key, base_id, "");
        let mapping = mapping.clone();
        Self { airtable, mapping }
    }

    /// Retrieves all cronjob records from the Airtable "WatchList" table.
    ///
    /// Returns a vector of `Record<Cronjob>` containing the fields "Symbol".
    ///
    /// # Errors
    ///
    /// Returns an error if fetching records from Airtable fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example(portal: &Portal) -> anyhow::Result<()> {
    /// let watchlist = portal.WatchList().await?;
    /// for record in watchlist {
    ///     println!("{:?}", record.fields);
    /// }
    /// # Ok(())
    /// # }
    /// Fetches all records from the "WatchList" table in Airtable, retrieving only the "Symbol" field.
    ///
    /// Returns a vector of `Record<WatchList>` on success. If fetching fails, returns an error with context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::{Portal, WatchList};
    /// # async fn example() -> anyhow::Result<()> {
    /// let portal = Portal::new("api_key", "base_id");
    /// let records = portal.watchlist().await?;
    /// for record in records {
    ///     if let Some(symbol) = &record.fields.symbol {
    ///         println!("Symbol: {}", symbol);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

    /// Retrieves all cronjob records from the Airtable "Cronjob" table.
    ///
    /// Returns a vector of `Record<Cronjob>` containing the fields "Crontime", "Route", "Timeout", and "Fuzzy".
    ///
    /// # Errors
    ///
    /// Returns an error if fetching records from Airtable fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example(portal: &Portal) -> anyhow::Result<()> {
    /// let cronjobs = portal.cronjob().await?;
    /// for record in cronjobs {
    ///     println!("{:?}", record.fields);
    /// }
    /// # Ok(())
    /// # }
    /// ```
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
