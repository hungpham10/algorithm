//! Example websocket server using Unix Socket.
//!
//! Run the server with
//! ```not_rust
//! cargo run -p example-websockets --bin example-websockets
//! ```

use std::os::unix::fs::PermissionsExt;
use axum::{
    body::Bytes,
    extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
    extract::connect_info::ConnectInfo,
    response::IntoResponse,
    routing::any,
    Router,
};
use axum_extra::TypedHeader;
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::UnixListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Custom ConnectInfo for Unix Socket
#[derive(Clone, Debug)]
struct UdsConnectInfo {
    peer_addr: Arc<tokio::net::unix::SocketAddr>,
    peer_cred: tokio::net::unix::UCred,
}

impl axum::extract::connect_info::Connected<axum::serve::IncomingStream<'_, UnixListener>> for UdsConnectInfo {
    fn connect_info(stream: axum::serve::IncomingStream<'_, UnixListener>) -> Self {
        let peer_addr = Arc::new(stream.io().peer_addr().unwrap());
        let peer_cred = stream.io().peer_cred().unwrap();
        Self { peer_addr, peer_cred }
    }
}

pub async fn run() -> std::io::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build router
    let app = Router::new()
        .route("/ws/test", any(ws_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // Unix socket setup
    let socket_path = PathBuf::from("/var/run/axum");

    // Cleanup old socket
    let _ = tokio::fs::remove_file(&socket_path).await;
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o666))?;

    println!("WebSocket server listening on Unix socket: {:?}", socket_path);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<UdsConnectInfo>(),
    )
    .await
}

/// WebSocket handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(info): ConnectInfo<UdsConnectInfo>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(ua)) = user_agent {
        ua.to_string()
    } else {
        String::from("Unknown browser")
    };

    println!(
        "`{user_agent}` connected via Unix socket. PID: {:?}, UID: {}, GID: {}",
        info.peer_cred.pid(),
        info.peer_cred.uid(),
        info.peer_cred.gid()
    );

    ws.on_upgrade(move |socket| handle_socket(socket, info))
}

/// Actual websocket handler
async fn handle_socket(mut socket: WebSocket, who: UdsConnectInfo) {
    // Send initial ping
    if socket
        .send(Message::Ping(Bytes::from_static(&[1, 2, 3])))
        .await
        .is_ok()
    {
        println!("Pinged Unix client...");
    } else {
        println!("Could not send ping!");
        return;
    }

    // ... (phần còn lại giữ nguyên như cũ)

    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, &who).is_break() {
                return;
            }
        } else {
            println!("Client abruptly disconnected");
            return;
        }
    }

    for i in 1..5 {
        if socket
            .send(Message::Text(format!("Hi {i} times!").into()))
            .await
            .is_err()
        {
            println!("Client abruptly disconnected");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        let n_msg = 20;
        for i in 0..n_msg {
            if sender
                .send(Message::Text(format!("Server message {i} ...").into()))
                .await
                .is_err()
            {
                return i;
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        n_msg
    });

    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            // In thực tế bạn có thể muốn log với thông tin từ UdsConnectInfo
            if process_message(msg, &who).is_break() {
                break;
            }
        }
        cnt
    });

    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => println!("{a} messages sent"),
                Err(e) => println!("Error sending: {e:?}"),
            }
            recv_task.abort();
        }
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(b) => println!("Received {b} messages"),
                Err(e) => println!("Error receiving: {e:?}"),
            }
            send_task.abort();
        }
    }

    println!("WebSocket connection closed");
}

// Helper function - updated to take UdsConnectInfo
fn process_message(msg: Message, who: &UdsConnectInfo) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => println!(">>> Unix client sent str: {t:?}"),
        Message::Binary(d) => println!(">>> Unix client sent {} bytes", d.len()),
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> Unix client sent close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            }
            return ControlFlow::Break(());
        }
        Message::Pong(v) => println!(">>> Unix client sent pong with {v:?}"),
        Message::Ping(v) => println!(">>> Unix client sent ping with {v:?}"),
    }
    ControlFlow::Continue(())
}
