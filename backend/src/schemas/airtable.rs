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
