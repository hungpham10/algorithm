pub mod admin;
pub mod investing;

use std::collections::HashMap;
use std::io::Error;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Json, Query, State};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response, Result};
use headers::Header;
use http::header;
use http::{HeaderName, HeaderValue};

use aws_sdk_s3::Client as S3Client;
use pprof::protos::Message;
use reqwest::Client as HttpClient;
use serde_json::Value;

use integration::QueryCandleSticks;
use models::entities::admin::Admin;
use models::entities::investing::Investing;
use models::resolver::Resolver;
use models::secret::Secret;
use schemas::reload::Reload;

use crate::vector::AxumRuntime;

#[derive(Debug)]
pub struct XTenantId(i64);

impl From<XTenantId> for i64 {
    fn from(tenant: XTenantId) -> Self {
        tenant.0
    }
}

impl Header for XTenantId {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("x-tenant-id");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> std::result::Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(headers::Error::invalid)?
            .to_str()
            .map_err(|_| headers::Error::invalid())?
            .parse::<i64>()
            .map_err(|_| headers::Error::invalid())?;

        Ok(XTenantId(value))
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_str(&self.0.to_string()).unwrap();
        values.extend(std::iter::once(value));
    }
}

#[derive(Clone)]
pub struct AppState {
    connector: Arc<Resolver>,
    secret: Arc<Secret>,
    runtime: Arc<AxumRuntime>,

    // @NOTE: entities configuration
    investing_entity: Arc<Investing>,
    admin_entity: Arc<Admin>,

    // @NOTE: integration services
    query_candlesticks: Arc<QueryCandleSticks>,
    s3: Arc<S3Client>,

    // @NOTE: investing
    investing_apis: Arc<HashMap<String, String>>,
}

impl AppState {
    pub async fn new(runtime: Arc<AxumRuntime>) -> Result<Self, Error> {
        let secret = Arc::new(Secret::new().await?);
        let connector = Arc::new(Resolver::new(secret.clone()).await?);
        let http_client = Arc::new(HttpClient::new());

        Ok(Self {
            // @NOTE: setup entity
            investing_entity: Arc::new(Investing::new(&connector)),
            admin_entity: Arc::new(Admin::new(&connector)),
            //chat_entity: Arc::new(Chat::new(&connector)),

            // @NOTE: setup integration
            s3: connector.s3(),
            query_candlesticks: Arc::new(QueryCandleSticks::new(http_client, 70)?),

            // @NOTE: shared components
            secret: secret.clone(),
            runtime,
            connector,

            // @NOTE: investing apis
            investing_apis: Arc::new(
                serde_json::from_str(secret.get("MAP_INVENSTING_API", "/").await?.as_str())
                    .map_err(|error| {
                        Error::other(format!(
                            "Failed to parse investing api from config: {}",
                            error
                        ))
                    })?,
            ),
        })
    }
}

impl Reload for AppState {
    fn reload(&self) -> Result<(), Error> {
        Ok(())
    }

    fn keys(&self) -> Vec<&str> {
        vec!["S3_BUCKET", "BROKER"]
    }
}

pub async fn health_check(State(_): State<AppState>) -> impl IntoResponse {
    "Success"
}

pub async fn reload(
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    for key in app_state.query_candlesticks.keys() {
        app_state
            .secret
            .force(key, "query_candlesticks")
            .await
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("fail to access key `{}`: {}", key, error),
                )
            })?;
    }

    app_state.query_candlesticks.reload().map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("fail to reload `query_candlesticks`: {}", error),
        )
    })?;

    for key in app_state.keys() {
        app_state
            .secret
            .force(key, "query_candlesticks")
            .await
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("fail to access key `{}`: {}", key, error),
                )
            })?;
    }
    app_state.reload().map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("fail to reload `app_state`: {}", error),
        )
    })?;
    Ok("Success")
}

pub async fn pprof(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let seconds = params
        .get("seconds")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

    let guard = match pprof::ProfilerGuard::new(100) {
        Ok(g) => g,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Profiler error: {e}"),
            )
                .into_response();
        }
    };

    tokio::time::sleep(Duration::from_secs(seconds)).await;

    let report = match guard.report().build() {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Report error: {e}"),
            )
                .into_response();
        }
    };

    if params.get("format").map(|s| s.as_str()) == Some("pb") {
        let profile = report.pprof().unwrap();
        let mut content = Vec::new();
        profile.write_to_vec(&mut content).unwrap();

        Response::builder()
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"profile.pb.gz\"",
            )
            .body(Body::from(content))
            .unwrap()
    } else {
        let mut body = Vec::new();
        match report.flamegraph(&mut body) {
            Ok(_) => Response::builder()
                .header(header::CONTENT_TYPE, "image/svg+xml")
                .body(Body::from(body))
                .unwrap(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Flamegraph error: {e}"),
            )
                .into_response(),
        }
    }
}

pub async fn into_stream(
    State(app_state): State<AppState>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let path = uri.path().to_string();

    app_state.runtime.handle(path, payload).await
}
