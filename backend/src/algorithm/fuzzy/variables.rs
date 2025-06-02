use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

use arrow::array::{ArrayRef, Float64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::Client;

use super::RuleError;
use chrono::Utc;

const DEFAULT_REGION: &str = "us-west-000";
const DEFAULT_ENDPOINT: &str = "https://s3.us-west-000.backblazeb2.com";

#[derive(Default, Clone)]
pub struct Variables {
    counter: usize,
    variables: HashMap<String, VecDeque<f64>>,
    variables_size: usize,
    buffers: HashMap<String, Vec<f64>>,
    buffers_size: usize,
    s3_bucket: Option<String>,
    s3_name: Option<String>,
    s3_client: Option<Client>,
}

impl Variables {
    /// Creates a new `Variables` instance with specified time series and buffer sizes.
    ///
    /// Initializes empty collections for variables and buffers, sets the update counter to zero, and leaves S3 integration unconfigured.
    ///
    /// # Parameters
    /// - `timeseries_size`: Maximum number of values to retain for each variable's time series.
    /// - `flush_after_incremental_size`: Number of buffered updates to accumulate before triggering a flush (if S3 is enabled).
    ///
    /// # Examples
    ///
    /// ```
    /// let vars = Variables::new(100, 50);
    /// assert_eq!(vars.variables_size, 100);
    /// assert_eq!(vars.buffers_size, 50);
    /// ```
    pub fn new(timeseries_size: usize, flush_after_incremental_size: usize) -> Self {
        Self {
            variables_size: timeseries_size,
            variables: HashMap::new(),
            buffers_size: flush_after_incremental_size,
            buffers: HashMap::new(),
            counter: 0,
            s3_bucket: None,
            s3_name: None,
            s3_client: None,
        }
    }

    /// Creates a new time series variable with the specified name.
    ///
    /// Returns an error if the variable already exists, or if S3 integration is enabled and creation is attempted after updates have started or after the initial variable set is established. When S3 is enabled, also initializes an incremental buffer for the variable.
    pub fn create(&mut self, name: &String) -> Result<(), RuleError> {
        if self.variables.contains_key(name) {
            return Err(RuleError {
                message: format!("Variable {} already exists", name),
            });
        }

        if !self.s3_client.is_none() {
            if self.counter > 0 {
                return Err(RuleError {
                    message: format!(
                        "Cannot create variable {} after {} updates",
                        name, self.counter
                    ),
                });
            }

            if self.variables.len() != self.buffers.len() {
                return Err(RuleError {
                    message: format!(
                        "Cannot create variable {} after {} variables",
                        name,
                        self.variables.len()
                    ),
                });
            }

            self.buffers
                .insert(name.clone(), Vec::with_capacity(self.buffers_size));
        }

        self.variables
            .insert(name.clone(), VecDeque::with_capacity(self.variables_size));
        Ok(())
    }

    /// Updates the specified variable with a new value and manages incremental buffering and flushing to S3 if configured.
    ///
    /// Adds the value to the variable's time series. If S3 integration is enabled, the value is also added to the incremental buffer, and a flush to S3 is triggered when the buffer is full and all columns are consistent. Returns the updated length of the variable's time series or an error if the operation fails.
    ///
    /// # Returns
    /// The new length of the variable's time series after the update.
    ///
    /// # Errors
    /// Returns a `RuleError` if the variable does not exist, if buffer consistency checks fail, or if flushing to S3 encounters an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Variables;
    /// # use tokio_test::block_on;
    /// let mut vars = Variables::new(10, 5);
    /// vars.create(&"temperature".to_string()).unwrap();
    /// let len = block_on(vars.update(&"temperature".to_string(), 23.5)).unwrap();
    /// assert_eq!(len, 1);
    /// ```
    pub async fn update(&mut self, name: &String, value: f64) -> Result<usize, RuleError> {
        let ret = self.update_variable(name, value).map_err(|e| e)?;

        if !self.s3_client.is_none() {
            let mut count_row_of_buffer = -1;
            let mut is_full_fill = true;

            for (column, buffer) in &self.buffers {
                if count_row_of_buffer < 0 {
                    count_row_of_buffer = buffer.len() as i32
                }

                let diff = (count_row_of_buffer - (buffer.len() as i32)).abs();

                if diff > 1 {
                    return Err(RuleError {
                        message: format!("Colume {} contain None value", column),
                    });
                }

                is_full_fill = diff == 0;
            }

            if is_full_fill && self.counter >= self.buffers_size {
                self.flush().await.map_err(|e| e)?;
            }

            self.update_buffer(&name, value).map_err(|e| e)?;
        }
        Ok(ret)
    }

    /// Retrieves the value of a variable at a specified index using an expression of the form `"variable[index]"`.
    ///
    /// Returns an error if the expression format is invalid, the variable does not exist, or the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vars = Variables::new(10, 5);
    /// vars.create(&"foo".to_string()).unwrap();
    /// vars.update(&"foo".to_string(), 42.0).unwrap();
    /// let value = vars.get_by_expr("foo[0]").unwrap();
    /// assert_eq!(value, 42.0);
    /// ```
    pub fn get_by_expr(&self, expr: &str) -> Result<f64, RuleError> {
        // Parse expression like "variable[index]"
        let parts: Vec<&str> = expr.split('[').collect();
        if parts.len() != 2 {
            return Err(RuleError {
                message: format!("Invalid expression format: {}", expr),
            });
        }

        let name = parts[0];
        let index_str = parts[1].trim_end_matches(']');
        let index = index_str.parse::<usize>().map_err(|_| RuleError {
            message: format!("Invalid index: {}", index_str),
        })?;

        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer.get(index).copied().ok_or_else(|| RuleError {
            message: format!("Index {} out of bounds for variable {}", index, name),
        })
    }

    /// Retrieves the value at the specified index for a given variable.
    ///
    /// Returns an error if the variable does not exist or the index is out of bounds.
    pub fn get_by_index(&self, name: &str, index: usize) -> Result<f64, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer.get(index).copied().ok_or_else(|| RuleError {
            message: format!("Index {} out of bounds for variable {}", index, name),
        })
    }

    /// Returns a vector of references to all variable names currently stored.
    pub fn list(&self) -> Vec<&String> {
        self.variables.keys().collect()
    }

    /// Removes all time series data for the specified variable.
    ///
    /// Returns an error if the variable does not exist.
    pub fn clear(&mut self, name: &str) -> Result<(), RuleError> {
        let buffer = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        buffer.clear();
        Ok(())
    }

    /// Returns the current length of the time series for the specified variable.
    ///
    /// # Errors
    ///
    /// Returns a `RuleError` if the variable does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vars = Variables::new(10, 5);
    /// vars.create(&"temperature".to_string()).unwrap();
    /// vars.update(&"temperature".to_string(), 23.5).await.unwrap();
    /// assert_eq!(vars.len("temperature").unwrap(), 1);
    /// ```
    pub fn len(&self, name: &str) -> Result<usize, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        Ok(buffer.len())
    }

    /// Configures the Variables instance to use an S3-compatible storage for data flushing.
    ///
    /// Initializes the S3 client asynchronously with the specified bucket name, region, and endpoint.
    /// If region or endpoint are not provided, defaults are used.
    ///
    /// # Parameters
    /// - `name`: The S3 bucket name to use for uploads.
    /// - `region`: Optional AWS region; defaults to "us-west-000" if not specified.
    /// - `endpoint`: Optional S3 endpoint URL; defaults to a Backblaze B2 S3-compatible endpoint if not specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::Variables;
    /// # async fn example() {
    /// let mut vars = Variables::new(100, 10);
    /// vars.use_s3("my-bucket".to_string(), Some("us-west-2"), None).await;
    /// # }
    /// ```
    pub async fn use_s3(&mut self, name: String, region: Option<&str>, endpoint: Option<&str>) {
        let region_value = region
            .map(|r| r.to_string())
            .unwrap_or(DEFAULT_REGION.to_string());
        let endpoint_value = endpoint.unwrap_or(DEFAULT_ENDPOINT);
        let region_provider =
            RegionProviderChain::default_provider().or_else(Region::new(region_value));
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_value)
            .behavior_version(BehaviorVersion::latest())
            .load()
            .await;
        self.s3_client = Some(Client::new(&config));
        self.s3_bucket = Some(name);
    }

    /// Updates the incremental buffer for the specified variable with a new value at the current buffer index.
    ///
    /// Returns an error if the variable does not have an associated buffer. Increments the global update counter after insertion.
    fn update_buffer(&mut self, name: &str, value: f64) -> Result<(), RuleError> {
        let buffer = self.buffers.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer[self.counter] = value;
        self.counter += 1;
        Ok(())
    }

    /// Updates the time series for the specified variable with a new value.
    ///
    /// Adds the new value to the front of the variable's time series, removing the oldest value if the series is at capacity. Increments the global update counter.
    ///
    /// # Parameters
    /// - `name`: The name of the variable to update.
    /// - `value`: The new value to add to the variable's time series.
    ///
    /// # Returns
    /// The new length of the variable's time series.
    ///
    /// # Errors
    /// Returns a `RuleError` if the specified variable does not exist.
    fn update_variable(&mut self, name: &str, value: f64) -> Result<usize, RuleError> {
        let variable = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        // Increase counter
        self.counter += 1;

        // Remove oldest value if variable is full
        if variable.len() >= self.variables_size {
            variable.pop_back();
        }

        // Add new value at front (most recent)
        variable.push_front(value);
        Ok(variable.len())
    }

    /// Asynchronously flushes buffered variable data to S3 as a Parquet file.
    ///
    /// Serializes the current contents of all variable buffers into an Apache Arrow RecordBatch,
    /// writes the data in Parquet format to an in-memory buffer, and uploads it to the configured
    /// S3 bucket with a timestamped object key. Resets the update counter after a successful upload.
    ///
    /// # Errors
    ///
    /// Returns a `RuleError` if the S3 client, bucket, or object name is not set, or if any step
    /// in batch creation, Parquet serialization, or S3 upload fails.
    async fn flush(&mut self) -> Result<(), RuleError> {
        let client = self.s3_client.as_ref().ok_or_else(|| RuleError {
            message: "S3 client not initialized".to_string(),
        })?;
        let name = self.s3_name.as_ref().ok_or_else(|| RuleError {
            message: "Variable name not set".to_string(),
        })?;
        let bucket = self.s3_bucket.as_ref().ok_or_else(|| RuleError {
            message: "Bucket name not set".to_string(),
        })?;
        let mapping = self
            .buffers
            .iter()
            .map(|(column, buffer)| (Field::new(column, DataType::Float64, false), buffer))
            .collect::<Vec<_>>();
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(
                mapping
                    .iter()
                    .map(|(arrow, _)| arrow.clone())
                    .collect::<Vec<_>>(),
            )),
            mapping
                .iter()
                .map(|&(_, buffer)| Arc::new(Float64Array::from(buffer.clone())) as ArrayRef)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| RuleError {
            message: format!("Failed to create batch: {}", e),
        })?;
        let props = WriterProperties::builder().build();

        let mut buffer = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), Some(props))
                .map_err(|e| RuleError {
                    message: format!("Failed to create writer: {}", e),
                })?;

            writer.write(&batch).map_err(|e| RuleError {
                message: format!("Failed to write batch: {}", e),
            })?;
        }

        client
            .put_object()
            .bucket(bucket)
            .key(&format!(
                "{}-{}.parquet",
                name,
                Utc::now().timestamp_millis()
            ))
            .body(buffer.into())
            .send()
            .await
            .map_err(|e| RuleError {
                message: format!("Failed to upload to S3: {}", e),
            })?;
        self.counter = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_creation() {
        let mut vars = Variables::new(5);
        assert!(vars.create("test".to_string()).is_ok());
        assert!(vars.create("test".to_string()).is_err());
    }

    #[test]
    fn test_variable_update() {
        let mut vars = Variables::new(3);
        vars.create(&"test".to_string()).unwrap();

        assert_eq!(vars.update(&"test".to_string(), 1.0).unwrap(), 1);
        assert_eq!(vars.update(&"test".to_string(), 2.0).unwrap(), 2);
        assert_eq!(vars.update(&"test".to_string(), 3.0).unwrap(), 3);
        assert_eq!(vars.update(&"test".to_string(), 4.0).unwrap(), 3);

        assert_eq!(vars.get_by_expr("test[0]").unwrap(), 4.0);
        assert_eq!(vars.get_by_expr("test[1]").unwrap(), 3.0);
        assert_eq!(vars.get_by_expr("test[2]").unwrap(), 2.0);
    }

    #[test]
    fn test_variable_get() {
        let mut vars = Variables::new(2);
        vars.create("test".to_string()).unwrap();
        vars.update("test".to_string(), 1.0).unwrap();

        assert!(vars.get_by_expr("invalid").is_err());
        assert!(vars.get_by_expr("test[2]").is_err());
        assert!(vars.get_by_expr("test[invalid]").is_err());
        assert_eq!(vars.get_by_expr("test[0]").unwrap(), 1.0);
    }

    #[test]
    fn test_get_by_index() {
        let mut vars = Variables::new(3);
        vars.create("test".to_string()).unwrap();

        vars.update("test".to_string(), 1.0).unwrap();
        vars.update("test".to_string(), 2.0).unwrap();

        // Test successful cases
        assert_eq!(vars.get_by_index("test", 0).unwrap(), 2.0);
        assert_eq!(vars.get_by_index("test", 1).unwrap(), 1.0);

        // Test error cases
        assert!(vars.get_by_index("nonexistent", 0).is_err());
        assert!(vars.get_by_index("test", 5).is_err());
    }
}
