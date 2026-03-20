use std::collections::HashMap;
use std::future::Future;
use std::io::Error;
use std::sync::Arc;

use axum::{Router, http::StatusCode, routing::MethodRouter};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use algorithm::AhoCorasick;
use vector_runtime::{Component, Event, Message, Runtime};

pub struct AxumRouter {
    mapping: RwLock<HashMap<String, String>>,
    engine: RwLock<AhoCorasick>,
}

impl AxumRouter {
    pub fn new() -> Self {
        Self {
            mapping: RwLock::new(HashMap::new()),
            engine: RwLock::new(AhoCorasick::new()),
        }
    }

    pub async fn route(&self, path: String, node: String) -> Result<(), Error> {
        let mut mapping = self.mapping.write().await;
        let mut engine = self.engine.write().await;

        mapping.insert(path.clone(), node);
        engine.add(path);
        Ok(())
    }

    pub async fn lookup(&self, path: &String) -> Option<String> {
        let engine = self.engine.read().await;

        if engine.similar(path) {
            return self.mapping.read().await.get(path).cloned();
        }
        None
    }
}

pub struct AxumRuntime {
    storage: Arc<AxumRouter>,
    runtime: Arc<RwLock<Runtime>>,
}

impl AxumRuntime {
    pub fn new(runtime: Arc<RwLock<Runtime>>, storage: Arc<AxumRouter>) -> Self {
        Self { runtime, storage }
    }

    pub async fn reload(&self, components: Vec<Arc<dyn Component>>) -> Result<(), Error> {
        self.runtime.write().await.reload(components)
    }

    pub async fn start<F, Fut>(&self, handler: F) -> Result<JoinHandle<()>, Error>
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut rt = self.runtime.write().await;
        rt.start(handler)
    }

    pub async fn stop(&self) -> Result<(), Error> {
        self.runtime.read().await.stop()
    }

    pub async fn wait_for_shutdown(&self) -> Result<(), Error> {
        self.runtime.read().await.wait_for_shutdown().await
    }

    pub async fn handle(&self, path: String, body: Value) -> (StatusCode, String) {
        let node_name = match self.storage.lookup(&path).await {
            Some(node) => node,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    format!("No route mapping found for path: {path}"),
                );
            }
        };

        match self
            .runtime
            .read()
            .await
            .inject(node_name.clone(), Message { payload: body })
            .await
        {
            Ok(_) => (
                StatusCode::OK,
                format!("Message successfully injected into node: {node_name}"),
            ),
            Err(error) => (
                StatusCode::BAD_GATEWAY,
                format!("Failed to inject message into node {node_name}: {error}"),
            ),
        }
    }
}

pub struct AxumBuilder {
    storage: Arc<AxumRouter>,
    runtime: Option<Arc<RwLock<Runtime>>>,
}

impl AxumBuilder {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(AxumRouter::new()),
            runtime: None,
        }
    }

    pub async fn route(self, path: &str, node: &str) -> Result<Self, Error> {
        self.storage
            .route(path.to_string(), node.to_string())
            .await?;
        Ok(self)
    }

    pub async fn build<F, S>(self, axum_router: &mut Router<S>, mut register_cb: F) -> AxumRuntime
    where
        S: Sync + Send + Clone + 'static,
        F: FnMut(&String) -> MethodRouter<S>,
    {
        let runtime = AxumRuntime::new(
            self.runtime
                .unwrap_or_else(|| Arc::new(RwLock::new(Runtime::new()))),
            self.storage.clone(),
        );

        self.storage.mapping.read().await.keys().for_each(|path| {
            *axum_router =
                std::mem::replace(axum_router, Router::new()).route(&path, register_cb(&path));
        });

        runtime
    }
}
