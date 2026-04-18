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

use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tonic::metadata::MetadataMap;
use tracing_opentelemetry::layer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use crate::api::admin;
use crate::vector::{AxumBuilder, AxumRuntime};
//use crate::api::chat;
use crate::api::investing;
use crate::api::{AppState, health_check, into_stream, pprof, reload};

fn init_telemetry() -> Option<(SdkTracerProvider, SdkMeterProvider)> {
    let trace_endpoint = std::env::var("OTEL_TRACE_EXPORTER_OTLP_ENDPOINT").unwrap_or_default();
    let metric_endpoint = std::env::var("OTEL_METRIC_EXPORTER_OTLP_ENDPOINT").unwrap_or_default();
    if trace_endpoint.is_empty() || metric_endpoint.is_empty() {
        return None;
    }

    let mut headers = std::collections::HashMap::new();
    let mut metadata = MetadataMap::with_capacity(1);

    if let Some((key, value)) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS")
        .unwrap_or_default()
        .split_once('=')
    {
        headers.insert(key.to_string(), value.to_string());
    }

    let uptrace_dsn = std::env::var("UPTRACE_DSN").unwrap_or_default();
    if !uptrace_dsn.is_empty() {
        metadata.insert("uptrace-dsn", uptrace_dsn.parse().unwrap());
    }

    let resource = Resource::builder()
        .with_attributes(vec![
            KeyValue::new("service.name", "universal-gateway"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])
        .build();

    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(trace_endpoint) // Grafana Cloud yêu cầu path cụ thể
        .with_metadata(metadata.clone())
        .build()
        .expect("Failed to create exporter");

    let trace_provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_resource(resource.clone())
        .build();
    global::set_tracer_provider(trace_provider.clone());

    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(metric_endpoint)
        .with_metadata(metadata)
        .build()
        .expect("Failed to create metric exporter");

    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(resource)
        .build();
    global::set_meter_provider(meter_provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());

    let telemetry_layer = layer().with_tracer(opentelemetry::trace::TracerProvider::tracer(
        &trace_provider,
        "universal-gateway",
    ));

    tracing_subscriber::registry()
        .with(
            EnvFilter::new("info")
                .add_directive("sqlx::query=info".parse().unwrap())
                .add_directive("tower_http=info".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .init();

    Some((trace_provider, meter_provider))
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
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
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
    let telemetry_guards = init_telemetry();
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

    if let Some((tp, mp)) = telemetry_guards {
        let _ = tp.force_flush();
        let _ = mp.force_flush();
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
