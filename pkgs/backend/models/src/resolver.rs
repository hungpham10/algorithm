use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;

use aws_config::{
    BehaviorVersion, Region, meta::region::RegionProviderChain, timeout::TimeoutConfig,
};
use aws_sdk_s3::Client as S3Client;

use url::Url;

use crate::secret::Secret;
use amqprs::connection::{Connection as StreamClient, OpenConnectionArguments};
use redis::Client as CacheClient;
use redis::aio::MultiplexedConnection;
use sea_orm::{ConnectOptions, Database, DatabaseConnection as DbClient};

#[derive(Clone)]
pub struct Resolver {
    streams: Vec<Arc<StreamClient>>,
    caches: Vec<MultiplexedConnection>,
    dbs: Vec<DbClient>,
    s3_client: Arc<S3Client>,
}

impl Resolver {
    pub async fn new(secret: Arc<Secret>) -> Result<Self, Error> {
        let mut streams = Vec::new();
        let mut caches = Vec::new();
        let mut dbs = Vec::new();

        let mysql_host = std::env::var("MYSQL_HOST").unwrap_or_else(|_| "".to_string());
        let mysql_port = std::env::var("MYSQL_PORT").unwrap_or_else(|_| "".to_string());
        let mysql_password = std::env::var("MYSQL_PASSWORD").unwrap_or_else(|_| "".to_string());
        let mysql_user = std::env::var("MYSQL_USER").unwrap_or_else(|_| "".to_string());
        let mysql_db = std::env::var("MYSQL_DATABASE").unwrap_or_else(|_| "".to_string());

        let db_dsn = secret.get("MYSQL_DSN", "/").await.unwrap_or(format!(
            "mysql://{}:{}@{}:{}/{}",
            mysql_user, mysql_password, mysql_host, mysql_port, mysql_db,
        ));

        for dsn in db_dsn.split(",") {
            let mut opt = ConnectOptions::new(dsn.to_string());

            opt.max_connections(100)
                .min_connections(5)
                .connect_timeout(Duration::from_secs(8))
                .idle_timeout(Duration::from_secs(8))
                .max_lifetime(Duration::from_secs(8))
                .sqlx_logging(true)
                .sqlx_logging_level(log::LevelFilter::Info);

            if let Ok(conn) = Database::connect(opt).await {
                dbs.push(conn);
            }
        }

        let s3_endpoint = secret.get("S3_ENDPOINT", "/").await.unwrap_or_default();
        let s3_region = secret.get("S3_REGION", "/").await.unwrap_or_default();
        let s3_client = Arc::new(S3Client::new(
            &(aws_config::defaults(BehaviorVersion::latest())
                .timeout_config(
                    TimeoutConfig::builder()
                        .operation_timeout(Duration::from_secs(30))
                        .operation_attempt_timeout(Duration::from_millis(10000))
                        .build(),
                )
                .region(
                    RegionProviderChain::first_try(Region::new(s3_region.clone()))
                        .or_default_provider(),
                )
                .endpoint_url(s3_endpoint.clone())
                .load()
                .await),
        ));
        let _ = secret.get("S3_BUCKET", "/").await.unwrap_or_default();

        let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "".to_string());
        let redis_port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "".to_string());
        let redis_password = std::env::var("REDIS_PASSWORD").unwrap_or_else(|_| "".to_string());
        let redis_username = std::env::var("REDIS_USERNAME").unwrap_or_else(|_| "".to_string());

        let redis_dsn = secret.get("REDIS_DSN", "/").await.unwrap_or(format!(
            "redis://{redis_username}:{redis_password}@{redis_host}:{redis_port}",
        ));

        for dsn in redis_dsn.split(",") {
            let client = CacheClient::open(dsn).map_err(|error| {
                Error::other(format!("New redis client to {dsn} failed: {error}"))
            })?;

            let conn = client
                .get_multiplexed_async_connection()
                .await
                .map_err(|error| Error::other(format!("Connect to {dsn} failed: {error}")))?;

            caches.push(conn);
        }
        for dsn in secret.get("LAVINMQ_DSN", "/").await?.split(",") {
            if let Ok(parsed) = Url::parse(dsn) {
                let mut args = OpenConnectionArguments::new(
                    parsed.host_str().ok_or_else(|| {
                        Error::new(ErrorKind::InvalidInput, "Invalid LAVINMQ_DSN")
                    })?,
                    parsed.port().unwrap_or(5672),
                    parsed.username(),
                    parsed.password().ok_or_else(|| {
                        Error::new(ErrorKind::InvalidInput, "Invalid LAVINMQ_DSN")
                    })?,
                );

                args.virtual_host(parsed.path().trim_start_matches('/'));
                streams.push(Arc::new(StreamClient::open(&args).await.map_err(
                    |error| {
                        Error::new(
                            ErrorKind::InvalidInput,
                            format!("Failed to connect to AMQP server: {}", error),
                        )
                    },
                )?))
            }
        }

        Ok(Self {
            caches,
            streams,
            dbs,
            s3_client,
        })
    }

    pub fn cache(&self, tenant_id: i64) -> MultiplexedConnection {
        self.caches
            .get((tenant_id % (self.caches.len() as i64)) as usize)
            .expect("Failed to get cache connection")
            .clone()
    }

    pub fn database(&self, tenant_id: i64) -> &DbClient {
        self.dbs
            .get((tenant_id % (self.dbs.len() as i64)) as usize)
            .unwrap_or_else(|| panic!("Failed to get database client for tenant_id: {}", tenant_id))
    }

    pub fn databases(&self) -> &Vec<DbClient> {
        &self.dbs
    }

    pub fn stream(&self, tenant_id: i64) -> (usize, &Arc<StreamClient>) {
        let sharding_id = (tenant_id % (self.streams.len() as i64)) as usize;

        (
            sharding_id,
            self.streams.get(sharding_id).unwrap_or_else(|| {
                panic!("Failed to get stream client for tenant_id: {}", tenant_id)
            }),
        )
    }

    pub fn streams(&self) -> &Vec<Arc<StreamClient>> {
        &self.streams
    }

    pub fn s3(&self) -> Arc<S3Client> {
        self.s3_client.clone()
    }
}
