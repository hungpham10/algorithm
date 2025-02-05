use std::sync::Arc;
use std::pin::Pin;

use bytes::Bytes;
use bytestring::ByteString;

use actix::prelude::*;
use actix::Addr;

use crate::actors::websocket::Rpc;

struct Renderer {
}

pub struct RendererActor {
}

impl RendererActor {
    fn new() -> Self {
        RendererActor{
            
        }
    }
}

impl Actor for RendererActor {
    type Context = Context<Self>;
}

#[derive(Clone)]
pub struct RendererRpc {
    actor: Arc<Addr<RendererActor>>,
}

impl Rpc for RendererRpc {
    fn boxing(&self) -> Pin<Box<dyn Rpc + Send + 'static>> {
        return Box::pin(RendererRpc{
            actor: self.actor.clone(),
        });
    }

    fn binary(&self, request: Bytes) -> Bytes {
        return request;
    }

    fn text(&self, request: ByteString) -> ByteString {
        return request;
    }
}

pub fn connect_to_renderer(
) -> (RendererRpc, Arc<Addr<RendererActor>>) {
    let actor  = Arc::new(
        RendererActor::new(
        )
        .start(),
    );
    return (RendererRpc{ actor: actor.clone() }, actor);
}
