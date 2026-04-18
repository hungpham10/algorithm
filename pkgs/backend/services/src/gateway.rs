use axum::{
    Router,
    body::Body,
    extract::connect_info,
    http::Request,
    routing::{get, post},
    serve::IncomingStream,
};

use axum::response::Json;
use axum_prometheus::PrometheusMetricLayer;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use tokio::net::UnixListener;
use tokio::net::unix::UCred;
use tokio::signal;
use tower::ServiceBuilder;
use vector_runtime::{Component, Event};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use std::fs;
use std::io::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tonic::metadata::MetadataMap;

use crate::api::admin;
use crate::vector::{AxumBuilder, AxumRuntime};
//use crate::api::chat;
use crate::api::investing;
use crate::api::{AppState, health_check, into_stream, pprof, reload};

fn init_tracer(dsn: String) -> Result<SdkTracerProvider, Box<dyn std::error::Error + Send + Sync>> {
    let mut metadata = MetadataMap::with_capacity(1);
    metadata.insert("uptrace-dsn", dsn.parse().unwrap());

    // Create OTLP span exporter
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint("https://api.uptrace.dev:4317")
        .with_metadata(metadata)
        .with_timeout(Duration::from_secs(10))
        .build()?;

    // Build the tracer provider
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder() // Sử dụng builder thay vì new
                .with_attributes(vec![
                    KeyValue::new("service.name", "universal-gateway"),
                    KeyValue::new("service.version", "1.0.0"),
                ])
                .build(),
        )
        .build();

    Ok(provider)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct UdsConnectInfo {
    peer_addr: Arc<tokio::net::unix::SocketAddr>,
    peer_cred: UCred,
}

impl connect_info::Connected<IncomingStream<'_, UnixListener>> for UdsConnectInfo {
    fn connect_info(stream: IncomingStream<'_, UnixListener>) -> Self {
        let peer_addr = stream.io().peer_addr().unwrap();
        let peer_cred = stream.io().peer_cred().unwrap();
        Self {
            peer_addr: Arc::new(peer_addr),
            peer_cred,
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(),
    nest(
        (path = "/api/investing", api = investing::InvestingApi)
    ),
    info(
        title = "Investing API Documentation",
        version = "1.0.0",
        description = "Api for serving investing solution and property management",
        license(
            name = "Proprietary / All Rights Reserved",
        )
    ),
)]
struct InvestingApiDoc;

async fn investing_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(InvestingApiDoc::openapi())
}

#[derive(OpenApi)]
#[openapi(
    paths(),
    nest(
        (path = "/api/admin", api = admin::AdminApi)
    ),
    info(
        title = "Admin API Documentation",
        version = "1.0.0",
        description = "Api for serving website admin",
        license(
            name = "Proprietary / All Rights Reserved",
        )
    ),
)]
struct AdminApiDoc;

async fn admin_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(AdminApiDoc::openapi())
}

pub async fn routes(
    components: Vec<Arc<dyn Component>>,
) -> Result<(Router, Arc<AxumRuntime>), Error> {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());

    // TODO: xem thử có cách nào load cấu hình từ yaml bên ngoài luôn đươc không
    let mut router = Router::new()
        .route("/health", get(health_check))
        .route("/reload", post(reload))
        .route("/debug/pprof/profile", get(pprof))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .route("/docs/investing/openapi.json", get(investing_openapi))
        .route("/docs/admin/openapi.json", get(admin_openapi))
        .nest("/api/admin", admin::routes())
        .nest("/api/investing", investing::routes())
        .merge(SwaggerUi::new("/swagger-ui").url("/docs/openapi.json", InvestingApiDoc::openapi()));

    let runtime = Arc::new(
        AxumBuilder::new()
            .route("/api/chat/v1/facebook/webhook", "facebook_source")
            .await?
            .route("/api/chat/v1/slack/webhook", "slack_source")
            .await?
            .build(&mut router, |_| post(into_stream))
            .await,
    );

    runtime.reload(components).await?;
    runtime
        .start(|event| async move {
            match event {
                Event::Minor((id, error)) => println!("Minor error in node {id}: {error}"),
                Event::Major((id, error)) => println!("Major error in node {id}: {error}"),
                Event::Panic((id, error)) => println!("Panic in node {id}: {error}"),
            }
        })
        .await?;

    let router = router
        .with_state(AppState::new(runtime.clone()).await?)
        .layer(prometheus_layer);

    let final_router = if environment == "prod" {
        router.layer(
            ServiceBuilder::new()
                .layer(SentryHttpLayer::new().enable_transaction())
                .layer(NewSentryLayer::<Request<Body>>::new_from_top()),
        )
    } else {
        router
    };

    Ok((final_router, runtime))
}

pub async fn run() -> std::io::Result<()> {
    let uptrace_dsn = std::env::var("UPTRACE_DSN").unwrap_or_default();
    let provider = if !uptrace_dsn.is_empty() {
        let provider = init_tracer(uptrace_dsn).expect("Failed to init tracer");

        global::set_tracer_provider(provider.clone());
        global::set_text_map_propagator(TraceContextPropagator::new());
        Some(provider)
    } else {
        None
    };

    let path = PathBuf::from("/var/run/axum");
    let _ = tokio::fs::remove_file(&path).await;
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;

    // Khởi tạo toàn bộ router và streamer trong một lần gọi
    // Bạn có thể truyền components ban đầu vào đây
    let (router, streamer) = routes(Vec::new()).await?;

    let make_service = router.into_make_service_with_connect_info::<UdsConnectInfo>();
    let usx = UnixListener::bind(path.clone())?;

    fs::set_permissions(&path, fs::Permissions::from_mode(0o666))?;

    println!("Server starting on Unix Socket: {:?}", path);

    let result = axum::serve(usx, make_service)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    // Shutdown sạch sẽ
    streamer.stop().await?;
    streamer.wait_for_shutdown().await?;

    if let Some(p) = provider {
        p.force_flush().unwrap();
    }

    result
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
        },
        _ = terminate => {
        },
    }
}
