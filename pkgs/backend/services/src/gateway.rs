use axum::{
    Router,
    body::Body,
    extract::connect_info,
    http::Request,
    routing::{get, post},
    serve::IncomingStream,
};

use axum_prometheus::PrometheusMetricLayer;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use tokio::net::UnixListener;
use tokio::net::unix::UCred;
use tokio::signal;
use tower::ServiceBuilder;
use vector_runtime::{Component, Event};

use std::fs;
use std::io::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use crate::api::admin;
use crate::vector::{AxumBuilder, AxumRuntime};
//use crate::api::chat;
use crate::api::investing;
use crate::api::{AppState, health_check, into_stream, pprof, reload};

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

pub async fn routes(
    components: Vec<Arc<dyn Component>>,
) -> Result<(Router, Arc<AxumRuntime>), Error> {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());

    // @TODO: xem thử có cách nào load cấu hình từ yaml bên ngoài luôn đươc không
    let mut router = Router::new()
        .route("/health", get(health_check))
        .route("/reload", post(reload))
        .route("/debug/pprof/profile", get(pprof))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .nest("/api/admin", admin::routes())
        .nest("/api/investing", investing::routes());

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
