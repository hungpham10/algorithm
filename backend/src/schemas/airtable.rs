use airtable_api::{Airtable, Record};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct WatchList {
    #[serde(rename = "Symbol")]
    pub symbol: Option<String>,
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

pub struct Portal {
    airtable: Airtable,
}

impl Portal {
    /// Creates a new `Portal` instance configured to access a specific Airtable base.
    ///
    /// # Examples
    ///
    /// ```
    /// let portal = Portal::new("your_api_key", "your_base_id");
    /// ```
    pub fn new(api_key: &str, base_id: &str) -> Self {
        let airtable = Airtable::new(api_key, base_id, "");
        Self { airtable }
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
        self.airtable
            .list_records("WatchList", "Watch", vec!["Symbol"])
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
        self.airtable
            .list_records(
                "Cronjob",
                "Cron",
                vec!["Crontime", "Route", "Timeout", "Fuzzy"],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch Cronjobs from Airtable: {}", e))
    }
}
