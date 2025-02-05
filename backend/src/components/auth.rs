use std::sync::Arc;
use std::pin::Pin;

use bytes::Bytes;
use bytestring::ByteString;

use actix::prelude::*;
use actix::Addr;

use crate::actors::websocket::Rpc;

struct Auth {
}

pub struct AuthActor {
}

impl AuthActor {
    fn new() -> Self {
        AuthActor{
            
        }
    }
}

impl Actor for AuthActor {
    type Context = Context<Self>;
}

#[derive(Clone)]
pub struct AuthRpc {
    actor: Arc<Addr<AuthActor>>,
}

impl Rpc for AuthRpc {
    fn boxing(&self) -> Pin<Box<dyn Rpc + Send + 'static>> {
        return Box::pin(AuthRpc{
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

pub fn connect_to_auth(
) -> (AuthRpc, Arc<Addr<AuthActor>>) {
    let actor  = Arc::new(
        AuthActor::new(
        )
        .start(),
    );
    return (AuthRpc{ actor: actor.clone() }, actor);
}
