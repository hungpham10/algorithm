use std::fs;
use std::io::Cursor;
use std::path::Path;
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
    s3_bucket: Option<String>,
    s3_client: Option<Arc<Client>>,
    vps_vars: Arc<Mutex<Variables>>,
    tcbs_vars: Arc<Mutex<Variables>>,
}

#[pymethods]
impl Datastore {
    #[new]
    fn new(vps_memory_size: usize, tcbs_memory_size: usize) -> PyResult<Self> {
        dotenvy::dotenv().ok();

        let s3_region = match std::env::var("S3_REGION") {
            Ok(s3_region) => Some(s3_region),
            Err(_) => None,
        };
        let s3_endpoint = match std::env::var("S3_ENDPOINT") {
            Ok(s3_endpoint) => Some(s3_endpoint),
            Err(_) => None,
        };
        let s3_bucket = match std::env::var("S3_BUCKET") {
            Ok(s3_bucket) => Some(s3_bucket),
            Err(_) => None,
        };

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
        match &self.s3_client {
            Some(s3_client) => {
                let client = s3_client.clone();
                let bucket = match &self.s3_bucket {
                    Some(s3_bucket) => s3_bucket,
                    None => {
                        return Err(PyRuntimeError::new_err("Not supported".to_string()));
                    }
                };

                Ok(actix_rt::Runtime::new()
                    .unwrap()
                    .block_on(async move {
                        Self::list_in_async(client.clone(), bucket.clone(), date.clone()).await
                    })
                    .map_err(|error| {
                        PyRuntimeError::new_err(format!("Failed to list: {}", error))
                    })?)
            }
            None => {
                let path = Path::new(&date);

                if !path.is_dir() {
                    return Err(PyRuntimeError::new_err(format!("{} not exist", date)));
                }

                Ok(fs::read_dir(path)
                    .map_err(|error| {
                        PyRuntimeError::new_err(format!("Failed to scan {}: {}", date, error))
                    })?
                    .filter_map(|it| {
                        if let Ok(it) = it {
                            let path = it.path();

                            if path.is_file() {
                                if let Some(file_name) = path.file_name() {
                                    if let Some(file_name) = file_name.to_str() {
                                        return Some(file_name.to_string());
                                    }
                                }
                            }
                        }

                        return None;
                    })
                    .collect::<Vec<_>>())
            }
        }
    }

    fn read(&self, file: String) -> PyResult<PyDataFrame> {
        Ok(PyDataFrame(match &self.s3_client {
            Some(s3_client) => {
                let client = s3_client.clone();
                let bucket = match &self.s3_bucket {
                    Some(s3_bucket) => s3_bucket,
                    None => {
                        return Err(PyRuntimeError::new_err("Not supported".to_string()));
                    }
                };

                actix_rt::Runtime::new()
                    .unwrap()
                    .block_on(async move {
                        Self::read_parquet_in_sync(client.clone(), bucket.clone(), file.clone())
                            .await
                    })
                    .map_err(|error| {
                        PyRuntimeError::new_err(format!("Failed to read: {}", error))
                    })?
            }
            None => {
                let path = Path::new(&file);
                if !path.exists() || !path.is_file() {
                    return Err(PyRuntimeError::new_err(format!(
                        "File {} does not exist or is not a file",
                        file
                    )));
                }

                let file = fs::File::open(path).map_err(|error| {
                    PyRuntimeError::new_err(format!("Failed to open file: {}", error))
                })?;

                ParquetReader::new(file).finish().map_err(|error| {
                    PyRuntimeError::new_err(format!("Failed to read parquet file: {}", error))
                })?
            }
        }))
    }

    fn delete(&self, file: String) -> PyResult<Vec<bool>> {
        match &self.s3_client {
            Some(s3_client) => {
                let client = s3_client.clone();
                let bucket = match &self.s3_bucket {
                    Some(s3_bucket) => s3_bucket,
                    None => {
                        return Err(PyRuntimeError::new_err(
                            "S3 bucket not configured".to_string(),
                        ));
                    }
                };

                Ok(actix_rt::Runtime::new()
                    .unwrap()
                    .block_on(async move {
                        Self::delete_in_async(client.clone(), bucket.clone(), file.clone()).await
                    })
                    .map_err(|error| {
                        PyRuntimeError::new_err(format!("Failed to delete: {}", error))
                    })?)
            }
            None => {
                let path = Path::new(&file);
                if !path.exists() {
                    return Err(PyRuntimeError::new_err(format!(
                        "File {} does not exist",
                        file
                    )));
                }

                fs::remove_file(path).map_err(|error| {
                    PyRuntimeError::new_err(format!("Failed to delete file: {}", error))
                })?;
                Ok(vec![true])
            }
        }
    }
}

impl Datastore {
    pub fn tcbs(&self) -> Arc<Mutex<Variables>> {
        self.tcbs_vars.clone()
    }

    pub fn vps(&self) -> Arc<Mutex<Variables>> {
        self.vps_vars.clone()
    }

    async fn new_s3_client_async(
        region: &Option<String>,
        endpoint: &Option<String>,
    ) -> Option<Arc<Client>> {
        let region_provider = Region::new(match region {
            Some(region) => region.clone(),
            None => return None,
        });
        let config = aws_config::defaults(BehaviorVersion::latest())
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(5))
                    .operation_attempt_timeout(Duration::from_millis(1500))
                    .build(),
            )
            .region(region_provider)
            .endpoint_url(match endpoint {
                Some(endpoint) => endpoint.clone(),
                None => return None,
            })
            .load()
            .await;
        Some(Arc::new(Client::new(&config)))
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

    async fn delete_in_async(
        client: Arc<Client>,
        bucket: String,
        file: String,
    ) -> Result<Vec<bool>> {
        client
            .delete_object()
            .bucket(bucket)
            .key(file)
            .send()
            .await?;
        Ok(vec![true])
    }
}
