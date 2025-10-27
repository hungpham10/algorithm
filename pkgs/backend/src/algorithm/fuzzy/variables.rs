use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use parquet::arrow::arrow_reader::ParquetRecordBatchReader;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::{WriterProperties, WriterVersion};

use arrow::array::{ArrayRef, Float64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use aws_config::{
    meta::region::RegionProviderChain, timeout::TimeoutConfig, BehaviorVersion, Region,
};
use aws_sdk_s3::Client;

use super::RuleError;

const DEFAULT_SCOPE: &str = "default";
const DEFAULT_REGION: &str = "us-west-000";
const DEFAULT_ENDPOINT: &str = "https://s3.us-west-000.backblazeb2.com";

#[derive(Default, Clone)]
pub struct Variables {
    variables: HashMap<String, VecDeque<f64>>,
    variables_size: usize,
    buffers: HashMap<String, Vec<f64>>,
    buffers_size: usize,
    mapping: HashMap<String, Vec<String>>,
    s3_bucket: Option<String>,
    s3_name: Option<String>,
    s3_client: Option<Client>,
}

impl Variables {
    pub fn new(timeseries_size: usize, flush_after_incremental_size: usize) -> Self {
        Self {
            variables_size: timeseries_size,
            variables: HashMap::new(),
            buffers_size: flush_after_incremental_size,
            buffers: HashMap::new(),
            mapping: HashMap::new(),
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

    pub async fn read_from_fs(
        &mut self,
        file_path: &str,
    ) -> Result<(ParquetRecordBatchReader, i64), RuleError> {
        let file = File::open(&file_path).map_err(|e| RuleError {
            message: format!("Failed to open file {}: {}", file_path, e),
        })?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| RuleError {
            message: format!("Failed to create builder: {}", e),
        })?;

        let num_of_rows = builder.metadata().file_metadata().num_rows();

        Ok((
            builder.build().map_err(|e| RuleError {
                message: format!("Failed to create reader: {}", e),
            })?,
            num_of_rows,
        ))
    }

    pub async fn list_from_s3(&self, scope: &str, prefix: &str) -> Result<Vec<i64>, RuleError> {
        let client = self.s3_client.as_ref().ok_or_else(|| RuleError {
            message: "S3 client not initialized".to_string(),
        })?;
        let bucket = self.s3_bucket.as_ref().ok_or_else(|| RuleError {
            message: "Bucket name not set".to_string(),
        })?;

        let response = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| RuleError {
                message: format!("Failed to list S3 objects: {}", e),
            })?;

        if let Some(contents) = response.contents {
            let timestamps = contents
                .iter()
                .filter_map(|obj| {
                    obj.key.as_ref().and_then(|key| {
                        let parts: Vec<&str> = key.split('/').collect();

                        if parts.len() > 1 {
                            let filename = parts.last()?;

                            if filename.contains(scope) {
                                let timestamp_str =
                                    filename.split('-').last()?.split('.').next()?;
                                timestamp_str.parse::<i64>().ok()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<_>>();

            if timestamps.is_empty() {
                Err(RuleError {
                    message: format!("No valid timestamps found for scope '{}' in folder", scope),
                })
            } else {
                Ok(timestamps)
            }
        } else {
            Err(RuleError {
                message: format!("Folder is empty or deleted"),
            })
        }
    }

    pub async fn read_from_s3(
        &mut self,
        scope: &str,
        timestamp_millis: i64,
    ) -> Result<(ParquetRecordBatchReader, i64), RuleError> {
        let client = self.s3_client.as_ref().ok_or_else(|| RuleError {
            message: "S3 client not initialized".to_string(),
        })?;
        let bucket = self.s3_bucket.as_ref().ok_or_else(|| RuleError {
            message: "Bucket name not set".to_string(),
        })?;
        let folder = format!(
            "investing/{}",
            DateTime::<Utc>::from_timestamp(timestamp_millis / 1000, 0)
                .unwrap()
                .format("%Y-%m-%d")
        );

        let scope = scope.to_string();
        let response = client
            .get_object()
            .bucket(bucket)
            .key(&self.name_of_parquet(&folder, &scope, timestamp_millis)?)
            .send()
            .await
            .map_err(|e| RuleError {
                message: format!("Failed to get from S3: {}", e),
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| RuleError {
                message: format!("Failed to collect S3 object to bytes: {}", e),
            })?
            .into_bytes();
        let builder = ParquetRecordBatchReaderBuilder::try_new(bytes).map_err(|e| RuleError {
            message: format!("Failed to create builder: {}", e),
        })?;
        let num_of_rows = builder.metadata().file_metadata().num_rows();

        Ok((
            builder.build().map_err(|e| RuleError {
                message: format!("Failed to create reader: {}", e),
            })?,
            num_of_rows,
        ))
    }

    pub fn scope(&mut self, name: &str, columns: &[String]) {
        self.mapping.insert(
            name.to_string(),
            columns.iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn create(&mut self, name: &String) -> Result<(), RuleError> {
        if self.variables.contains_key(name) {
            return Err(RuleError {
                message: format!("Variable {} already exists", name),
            });
        }

        if self.s3_client.is_some() {
            if self.variables.len() != self.buffers.len() {
                return Err(RuleError {
                    message: format!("Cannot create variable {}", name),
                });
            }

            self.clean_all_buffer_and_insert_new_buffer(name);
        }

        self.variables.insert(
            name.to_owned(),
            VecDeque::with_capacity(self.variables_size),
        );
        Ok(())
    }

    fn clean_all_buffer_and_insert_new_buffer(&mut self, name: &String) {
        // @NOTE: This clears all existing buffer data
        for (_, buffer) in self.buffers.iter_mut() {
            buffer.clear();
        }

        self.buffers.insert(name.clone(), Vec::new());
    }

    fn get_scope_columns<'a>(&'a self, scope: &str) -> HashSet<&'a String> {
        match self.mapping.get(scope) {
            Some(mapping) => mapping.iter().collect(),
            None => self.variables.keys().collect(),
        }
    }

    pub async fn update(
        &mut self,
        scope: &String,
        name: &String,
        value: f64,
    ) -> Result<usize, RuleError> {
        self.update_with_controlled_flushing(scope, name, value, true)
            .await
    }

    async fn update_with_controlled_flushing(
        &mut self,
        scope: &String,
        name: &String,
        value: f64,
        use_s3_flushing: bool,
    ) -> Result<usize, RuleError> {
        let ret = self.update_variable(name, value).map_err(|e| e)?;

        if use_s3_flushing && !self.s3_client.is_none() {
            let mut count_row_of_buffer: Option<i32> = None;
            let mut is_full_fill = true;

            let mapping = self.get_scope_columns(scope);

            for (column, buffer) in &self.buffers {
                if !mapping.contains(column) {
                    continue;
                }

                if let Some(count_row_of_buffer) = count_row_of_buffer {
                    let diff = (count_row_of_buffer - (buffer.len() as i32)).abs();

                    if diff > 1 {
                        return Err(RuleError {
                            message: format!("Column {} contain None value", column),
                        });
                    }

                    if is_full_fill {
                        is_full_fill = diff == 0;
                    }
                } else {
                    count_row_of_buffer = Some(buffer.len() as i32);
                }
            }

            if is_full_fill && count_row_of_buffer.unwrap() >= self.buffers_size as i32 {
                let buffer = self.prepare_flushing(scope)?;

                self.do_flushing(buffer, scope).await?;
            }

            self.update_buffer(&name, value)?;
        }
        Ok(ret)
    }

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

    pub fn get_by_index(&self, name: &str, index: usize) -> Result<f64, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        buffer.get(index).copied().ok_or_else(|| RuleError {
            message: format!("Index {} out of bounds for variable {}", index, name),
        })
    }

    pub fn last(&self, name: &str) -> Result<f64, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        if let Some(val) = buffer.back() {
            Ok(*val)
        } else {
            Err(RuleError {
                message: format!("Variable {} is empty", name),
            })
        }
    }

    pub fn list(&self) -> Vec<&String> {
        self.variables.keys().collect()
    }

    pub fn clear(&mut self, name: &str) -> Result<(), RuleError> {
        let buffer = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        buffer.clear();

        Ok(())
    }

    pub fn clear_all(&mut self) {
        self.buffers.clear();
        self.variables.clear();
    }

    pub fn len(&self, name: &str) -> Result<usize, RuleError> {
        let buffer = self.variables.get(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;
        Ok(buffer.len())
    }

    pub async fn use_s3(
        &mut self,
        bucket: &str,
        name: &str,
        region: Option<&str>,
        endpoint: Option<&str>,
    ) {
        let region_value = region
            .map(|r| r.to_string())
            .unwrap_or(DEFAULT_REGION.to_string());
        let endpoint_value = endpoint.unwrap_or(DEFAULT_ENDPOINT);
        let region_provider = RegionProviderChain::first_try(Region::new(region_value.to_string()))
            .or_default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(30))
                    .operation_attempt_timeout(Duration::from_millis(10000))
                    .build(),
            )
            .region(region_provider)
            .endpoint_url(endpoint_value)
            .load()
            .await;
        self.s3_client = Some(Client::new(&config));
        self.s3_name = Some(name.to_string());
        self.s3_bucket = Some(bucket.to_string());
    }

    pub async fn flush_all(&mut self) -> Result<(), RuleError> {
        let scopes: Vec<String> = self.mapping.keys().map(|s| s.to_string()).collect();

        for scope in &scopes {
            self.flush(scope).await?
        }
        if scopes.len() == 0 {
            self.flush(DEFAULT_SCOPE).await?
        }
        Ok(())
    }

    pub async fn flush(&mut self, scope: &str) -> Result<(), RuleError> {
        if self.s3_client.is_none() {
            return Ok(());
        }

        let buffer = self.prepare_flushing(&scope.to_string())?;
        self.do_flushing(buffer, &scope.to_string()).await
    }

    fn update_buffer(&mut self, name: &str, value: f64) -> Result<(), RuleError> {
        let buffer = self.buffers.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Buffer {} not found", name),
        })?;

        buffer.push(value);
        Ok(())
    }

    fn update_variable(&mut self, name: &str, value: f64) -> Result<usize, RuleError> {
        let variable = self.variables.get_mut(name).ok_or_else(|| RuleError {
            message: format!("Variable {} not found", name),
        })?;

        if variable.len() >= self.variables_size {
            variable.pop_back();
        }

        variable.push_front(value);
        Ok(variable.len())
    }

    fn prepare_flushing(&mut self, scope: &String) -> Result<Vec<u8>, RuleError> {
        let scope = self.get_scope_columns(scope);

        let mapping = self
            .buffers
            .iter()
            .filter_map(|(column, buffer)| {
                if scope.contains(&column) && buffer.len() > 0 {
                    Some((Field::new(column, DataType::Float64, false), buffer))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // @NOTE: if no data, return soon to avoid creating empty parquet
        if mapping.len() == 0 {
            return Ok(Vec::new());
        }

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

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::default()))
            .set_writer_version(WriterVersion::PARQUET_2_0)
            .build();

        let mut buffer = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), Some(props))
                .map_err(|e| RuleError {
                    message: format!("Failed to create writer: {}", e),
                })?;

            writer.write(&batch).map_err(|e| RuleError {
                message: format!("Failed to write batch: {}", e),
            })?;
            writer.close().map_err(|e| RuleError {
                message: format!("Failed to close writer: {}", e),
            })?;
        }

        Ok(buffer)
    }

    fn name_of_parquet(
        &self,
        folder: &String,
        scope: &String,
        timestamp_millis: i64,
    ) -> Result<String, RuleError> {
        let name = self.s3_name.as_ref().ok_or_else(|| RuleError {
            message: "Variable name not set".to_string(),
        })?;

        Ok(format!(
            "{}/{}-{}-{}.parquet",
            folder, name, scope, timestamp_millis,
        ))
    }

    async fn do_flushing(&mut self, buffer: Vec<u8>, scope: &String) -> Result<(), RuleError> {
        let client = self.s3_client.as_ref().ok_or_else(|| RuleError {
            message: "S3 client not initialized".to_string(),
        })?;
        let bucket = self.s3_bucket.as_ref().ok_or_else(|| RuleError {
            message: "Bucket name not set".to_string(),
        })?;
        let folder = format!("investing/{}", Utc::now().format("%Y-%m-%d"));

        if buffer.len() > 0 {
            client
                .put_object()
                .bucket(bucket)
                .key(&self.name_of_parquet(&folder, scope, Utc::now().timestamp_millis())?)
                .body(buffer.into())
                .send()
                .await
                .map_err(|e| RuleError {
                    message: format!("Failed to upload to S3: {}", e),
                })?;

            self.clean_cache_after_flushing(scope);
        }
        Ok(())
    }

    fn clean_cache_after_flushing(&mut self, scope: &String) {
        let mapping: HashSet<String> = self
            .get_scope_columns(scope)
            .into_iter()
            .map(|column| column.clone())
            .collect();

        for (column, buffer) in self.buffers.iter_mut() {
            if !mapping.contains(column) {
                continue;
            }

            buffer.clear();
        }
    }
}
