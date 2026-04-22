use algorithm::{JsonQuery, LruCache};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde_json::Value;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use url::Url;

pub struct Api {
    clients: LruCache<String, ClientWithMiddleware, 32>,
}

impl Api {
    pub fn new(size: usize) -> Self {
        Self {
            clients: LruCache::new(size * 32),
        }
    }

    fn get_client(&self, url_str: &str) -> Result<(ClientWithMiddleware, String), Error> {
        let host = Url::parse(url_str)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid URL: {}", e)))?
            .host_str()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "URL has no host"))?
            .to_string();

        let client = self.clients.get(&host).unwrap_or_else(|| {
            let new_client = ClientBuilder::new(reqwest::Client::new())
                .with(TracingMiddleware::default())
                .build();

            self.clients.put(host.clone(), new_client.clone());
            new_client
        });

        Ok((client, host))
    }

    fn build_headers(&self, headers: &HashMap<String, String>) -> Result<HeaderMap, Error> {
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            let name = HeaderName::from_bytes(key.as_bytes()).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid header name: {}", key),
                )
            })?;
            let val = HeaderValue::from_str(value).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid header value: {}", value),
                )
            })?;
            header_map.insert(name, val);
        }
        Ok(header_map)
    }

    async fn parse_response(
        &self,
        response: reqwest::Response,
        parser: &Arc<JsonQuery>,
    ) -> Result<Vec<Value>, Error> {
        if !response.status().is_success() {
            return Err(Error::other(format!("HTTP Error: {}", response.status())));
        }

        // Đọc toàn bộ body thành JSON Value
        let json_data: Value = response.json().await.map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse JSON: {}", e),
            )
        })?;

        Ok(parser.execute(&json_data).into_iter().cloned().collect())
    }

    pub async fn create(
        &self,
        url: &str,
        parser: &Arc<JsonQuery>,
        headers: &HashMap<String, String>,
        body: Value,
    ) -> Result<Vec<Value>, Error> {
        let (client, _) = self.get_client(url)?;
        let header_map = self.build_headers(headers)?;

        let response = client
            .post(url)
            .headers(header_map)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::other(format!("Request failed: {}", e)))?;

        self.parse_response(response, parser).await
    }

    pub async fn read(
        &self,
        url: &str,
        parser: &Arc<JsonQuery>,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<Value>, Error> {
        let (client, _) = self.get_client(url)?;
        let header_map = self.build_headers(headers)?;

        let response = client
            .get(url)
            .headers(header_map)
            .send()
            .await
            .map_err(|e| Error::other(format!("Request failed: {}", e)))?;

        self.parse_response(response, parser).await
    }

    pub async fn update(
        &self,
        url: &str,
        parser: &Arc<JsonQuery>,
        headers: &HashMap<String, String>,
        body: Value,
    ) -> Result<Vec<Value>, Error> {
        let (client, _) = self.get_client(url)?;
        let header_map = self.build_headers(headers)?;

        let response = client
            .put(url)
            .headers(header_map)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::other(format!("Request failed: {}", e)))?;

        self.parse_response(response, parser).await
    }

    pub async fn delete(
        &self,
        url: &str,
        parser: &Arc<JsonQuery>,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<Value>, Error> {
        let (client, _) = self.get_client(url)?;
        let header_map = self.build_headers(headers)?;

        let response = client
            .delete(url)
            .headers(header_map)
            .send()
            .await
            .map_err(|e| Error::other(format!("Request failed: {}", e)))?;

        self.parse_response(response, parser).await
    }
}
