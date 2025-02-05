use std::sync::Arc;
use std::pin::Pin;
use std::time::{Duration, Instant};
use std::collections::BTreeMap;

use bytes::Bytes;
use bytestring::ByteString;

use actix::prelude::*;
use actix_web::{web, Resource, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws::{start, Message as WebsocketMessage, WebsocketContext, ProtocolError};

// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub trait Rpc {
    fn boxing(&self) -> Pin<Box<dyn Rpc + Send + 'static>>;
    fn binary(&self, request: Bytes) -> Bytes;
    fn text(&self, request: ByteString) -> ByteString;
}

struct SessionActor {
    handler: Pin<Box<dyn Rpc + Send + 'static>>,
    heartbeat: Instant,
}

impl Actor for SessionActor {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
    }
}

impl SessionActor {
    pub fn new(handler: Pin<Box<dyn Rpc + Send + 'static>>) -> Self {
        Self {
            handler: handler,
            heartbeat: Instant::now(),
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl StreamHandler<Result<WebsocketMessage, ProtocolError>> for SessionActor {
    fn handle(&mut self, msg: Result<WebsocketMessage, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(WebsocketMessage::Ping(data)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&data);
            }
            Ok(WebsocketMessage::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(WebsocketMessage::Text(req)) => {
                ctx.text(self.handler.text(req));
            },
            Ok(WebsocketMessage::Binary(req)) => {
                ctx.binary(self.handler.binary(req));
            },
            Ok(WebsocketMessage::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(WebsocketMessage::Nop) => (),
            _ => {
                ctx.stop();
            }
        }
    }
}

#[derive(Debug)]
pub struct WebsocketError {
    msg: String,
}

pub struct Websocket {
    handlers: BTreeMap<String, Pin<Box<dyn Rpc>>>,
}

impl Websocket {
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
        }
    }

    pub fn configure(
        &mut self, 
        path: &str, 
        handler: Pin<Box<dyn Rpc>>,
    ) -> Resource {
        self.put(path, handler);

        web::resource("/auth/v1/{user}")
            .route(web::get().to(handle_rpc_websocket))
    }
    
    fn put(&mut self, name: &str, handler: Pin<Box<dyn Rpc>>) {
        self.handlers.insert(name.to_string(), handler);
    }

    fn get(&self, name: &str) -> Result<SessionActor, WebsocketError> {
        if let Some(handler) = self.handlers.get(&name.to_string()) {
            Ok(SessionActor::new(handler.boxing()))
        } else {
            Err(WebsocketError { msg: format!("no handler for {}", name) })
        }
    }
}

async fn handle_rpc_websocket(
    request: HttpRequest,
    stream: web::Payload,
    websocket: web::Data<Arc<Websocket>>,
) -> Result<HttpResponse, Error> {
    match websocket.get(request.path()) {
        Ok(handler) => {
            start(handler, &request, stream)
        },
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.msg)),
    }
}

