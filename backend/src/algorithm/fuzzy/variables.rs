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
    /// Initializes empty collections for variables and buffers, and sets up configuration for optional S3 integration.
    ///
    /// # Parameters
    /// - `timeseries_size`: Maximum number of recent values to retain for each variable.
    /// - `flush_after_incremental_size`: Number of updates to buffer before triggering a flush (e.g., to S3).
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

    pub async fn new_with_s3(
        timeseries_size: usize,
        flush_after_incremental_size: usize,
        s3_bucket: &str,
        s3_name: &str,
        s3_region: Option<&str>,
        s3_endpoint: Option<&str>,
    ) -> Self {
        let mut vars = Self::new(timeseries_size, flush_after_incremental_size);

        vars.use_s3(s3_bucket, s3_name, s3_region, s3_endpoint)
            .await;
        vars
    }

    /// Creates a new time series variable with the specified name.
    ///
    /// Returns an error if the variable already exists. If S3 buffering is enabled, creation is only allowed before any updates have occurred and when the number of variables matches the number of buffers. Initializes both the time series and, if applicable, the buffer for the new variable.
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

            self.buffers.insert(name.clone(), Vec::new());
        }

        self.variables
            .insert(name.clone(), VecDeque::with_capacity(self.variables_size));
        Ok(())
    }

    /// Updates the specified variable with a new value and manages buffer flushing to S3 if enabled.
    ///
    /// Inserts the new value into the time series for the given variable, maintaining its fixed size. If S3 buffering is enabled, updates the corresponding buffer and triggers an asynchronous flush to S3 in Parquet format when all buffers are full and the flush threshold is reached. Returns the current length of the variable's time series after the update.
    ///
    /// # Errors
    ///
    /// Returns an error if the variable does not exist, if buffer consistency checks fail, or if flushing to S3 encounters an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Variables;
    /// # use tokio_test::block_on;
    /// let mut vars = Variables::new(3, 2);
    /// vars.create(&"temperature".to_string()).unwrap();
    /// let len = block_on(vars.update(&"temperature".to_string(), 25.0)).unwrap();
    /// assert_eq!(len, 1);
    /// ```
    pub async fn update(&mut self, name: &String, value: f64) -> Result<usize, RuleError> {
        let ret = self.update_variable(name, value).map_err(|e| e)?;

        if !self.s3_client.is_none() {
            let mut count_row_of_buffer: Option<i32> = None;
            let mut is_full_fill = true;

            for (column, buffer) in &self.buffers {
                if let Some(count_row_of_buffer) = count_row_of_buffer {
                    let diff = (count_row_of_buffer - (buffer.len() as i32)).abs();

                    if diff > 1 {
                        return Err(RuleError {
                            message: format!("Column {} contain None value", column),
                        });
                    }

                    is_full_fill = diff == 0;
                } else {
                    count_row_of_buffer = Some(buffer.len() as i32);
                }
            }

            if is_full_fill && self.counter >= self.buffers_size {
                self.flush().await.map_err(|e| e)?;
            }

            self.update_buffer(&name, value).map_err(|e| e)?;
        }
        Ok(ret)
    }

    /// Retrieves a value from a variable's time series using an expression of the form `"variable[index]"`.
    ///
    /// Returns an error if the expression format is invalid, the variable does not exist, or the index is out of bounds.
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

    /// Returns a vector of references to all variable names currently managed by the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vars = Variables::new(10, 5);
    /// vars.create(&"temperature".to_string()).unwrap();
    /// vars.create(&"humidity".to_string()).unwrap();
    /// let names = vars.list();
    /// assert!(names.contains(&&"temperature".to_string()));
    /// assert!(names.contains(&&"humidity".to_string()));
    /// ```
    pub fn list(&self) -> Vec<&String> {
        self.variables.keys().collect()
    }

    /// Removes all stored values for the specified variable.
    ///
    /// Returns an error if the variable does not exist.
    pub fn clear(&mut self, name: &str) -> Result<(), RuleError> {
        let buffer = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        buffer.clear();
        Ok(())
    }

    /// Returns the number of stored values for the specified variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the variable does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vars = Variables::new(5, 10);
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

    /// Configures the struct to use S3-compatible storage for buffered data uploads.
    ///
    /// Initializes the S3 client with the specified bucket, object name prefix, and optional region and endpoint.
    /// Subsequent flush operations will upload Parquet files to the configured S3 location.
    ///
    /// # Parameters
    /// - `name`: Prefix for S3 object names.
    /// - `bucket`: Name of the S3 bucket.
    /// - `region`: Optional AWS region; uses a default if not provided.
    /// - `endpoint`: Optional custom endpoint; uses a default if not provided.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Variables;
    /// # async fn example() {
    /// let mut vars = Variables::new(100, 10);
    /// vars.use_s3("mydata", "mybucket", Some("us-west-2"), None).await;
    /// # }
    /// ```
    pub async fn use_s3(
        &mut self,
        name: &str,
        bucket: &str,
        region: Option<&str>,
        endpoint: Option<&str>,
    ) {
        let region_value = region
            .map(|r| r.to_string())
            .unwrap_or(DEFAULT_REGION.to_string());
        let endpoint_value = endpoint.unwrap_or(DEFAULT_ENDPOINT);
        let region_provider =
            RegionProviderChain::default_provider().or_else(Region::new(region_value));
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_value)
            .load()
            .await;
        self.s3_client = Some(Client::new(&config));
        self.s3_name = Some(name.to_string());
        self.s3_bucket = Some(bucket.to_string());
    }

    /// Updates the buffer for the specified variable at the current counter index with a new value.
    ///
    /// Returns an error if the buffer for the given variable does not exist. Increments the global update counter after the update.
    fn update_buffer(&mut self, name: &str, value: f64) -> Result<(), RuleError> {
        let buffer = self.buffers.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer.push(value);
        self.counter += 1;
        Ok(())
    }

    /// Inserts a new value at the front of the specified variable's time series, removing the oldest value if the series is full.
    ///
    /// Returns the updated length of the variable's time series. Returns an error if the variable does not exist.
    fn update_variable(&mut self, name: &str, value: f64) -> Result<usize, RuleError> {
        let variable = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        // Remove oldest value if variable is full
        if variable.len() >= self.variables_size {
            variable.pop_back();
        }

        // Add new value at front (most recent)
        variable.push_front(value);
        Ok(variable.len())
    }

    /// Serializes buffered variable data to Parquet format and uploads it to the configured S3 bucket.
    ///
    /// Converts all variable buffers into an Apache Arrow `RecordBatch`, writes the batch to an in-memory Parquet file,
    /// and uploads the file to S3 using a timestamped key. Resets the update counter after a successful upload.
    ///
    /// # Errors
    ///
    /// Returns an error if the S3 client, bucket, or variable name is not configured, or if any step in batch creation,
    /// Parquet writing, or S3 upload fails.
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

        self.buffers
            .get_mut(name)
            .ok_or_else(|| RuleError {
                message: format!("Variable {} not found", name),
            })?
            .clear();
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
