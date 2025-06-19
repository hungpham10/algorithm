use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use polars::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

use anyhow::{anyhow, Result};

use aws_config::{
    meta::region::RegionProviderChain, timeout::TimeoutConfig, BehaviorVersion, Region,
};
use aws_sdk_s3::Client;

use crate::algorithm::fuzzy::Variables;

#[pyclass]
pub struct Datastore {
    s3_bucket: String,
    s3_client: Arc<Client>,
    vps_vars: Arc<Mutex<Variables>>,
    tcbs_vars: Arc<Mutex<Variables>>,
}

#[pymethods]
impl Datastore {
    #[new]
    fn new(vps_memory_size: usize, tcbs_memory_size: usize) -> PyResult<Self> {
        dotenvy::dotenv().ok();

        let s3_region = std::env::var("S3_REGION").map_err(|error| {
            PyRuntimeError::new_err(format!("Failed to get S3_REGION: {}", error))
        })?;
        let s3_endpoint = std::env::var("S3_ENDPOINT").map_err(|error| {
            PyRuntimeError::new_err(format!("Failed to get S3_ENDPOINT: {}", error))
        })?;
        let s3_bucket = std::env::var("S3_BUCKET").map_err(|error| {
            PyRuntimeError::new_err(format!("Failed to get S3_BUCKET: {}", error))
        })?;

        let s3_client = actix_rt::Runtime::new()
            .unwrap()
            .block_on(async move { Self::new_s3_client_async(&s3_region, &s3_endpoint).await });

        Ok(Datastore {
            s3_bucket,
            s3_client,
            vps_vars: Arc::new(Mutex::new(Variables::new(vps_memory_size, 0))),
            tcbs_vars: Arc::new(Mutex::new(Variables::new(tcbs_memory_size, 0))),
        })
    }

    fn list(&self, date: String) -> PyResult<Vec<String>> {
        let client = self.s3_client.clone();
        let bucket = self.s3_bucket.clone();

        Ok(actix_rt::Runtime::new()
            .unwrap()
            .block_on(async move {
                Self::list_in_async(client.clone(), bucket.clone(), date.clone()).await
            })
            .map_err(|error| PyRuntimeError::new_err(format!("Failed to list: {}", error)))?)
    }

    fn read(&self, file: String) -> PyResult<PyDataFrame> {
        let client = self.s3_client.clone();
        let bucket = self.s3_bucket.clone();

        Ok(PyDataFrame(
            actix_rt::Runtime::new()
                .unwrap()
                .block_on(async move {
                    Self::read_parquet_in_sync(client.clone(), bucket.clone(), file.clone()).await
                })
                .map_err(|error| PyRuntimeError::new_err(format!("Failed to read: {}", error)))?,
        ))
    }
}

impl Datastore {
    pub fn tcbs(&self) -> Arc<Mutex<Variables>> {
        self.tcbs_vars.clone()
    }

    pub fn vps(&self) -> Arc<Mutex<Variables>> {
        self.vps_vars.clone()
    }

    async fn new_s3_client_async(region: &String, endpoint: &String) -> Arc<Client> {
        let region_provider = Region::new(region.clone());
        let config = aws_config::defaults(BehaviorVersion::latest())
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(5))
                    .operation_attempt_timeout(Duration::from_millis(1500))
                    .build(),
            )
            .region(region_provider)
            .endpoint_url(endpoint)
            .load()
            .await;
        Arc::new(Client::new(&config))
    }

    async fn list_in_async(
        client: Arc<Client>,
        bucket: String,
        date: String,
    ) -> Result<Vec<String>> {
        let response = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(date)
            .send()
            .await?;

        if let Some(contents) = response.contents {
            Ok(contents
                .iter()
                .filter_map(|it| it.key.clone())
                .collect::<Vec<_>>())
        } else {
            Err(anyhow!("Folder is empty or deleted"))
        }
    }

    async fn read_parquet_in_sync(
        client: Arc<Client>,
        bucket: String,
        file: String,
    ) -> Result<DataFrame> {
        let response = client.get_object().bucket(bucket).key(file).send().await?;

        Ok(
            ParquetReader::new(Cursor::new(response.body.collect().await?.into_bytes()))
                .finish()?,
        )
    }
}
