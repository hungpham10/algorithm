use std::sync::Arc;
use std::pin::Pin;

use bytes::Bytes;
use bytestring::ByteString;

use actix::prelude::*;
use actix::Addr;

use crate::actors::websocket::Rpc;

struct Event {
}

pub struct EventActor {
}

impl EventActor {
    fn new() -> Self {
        EventActor{
            
        }
    }
}
impl Actor for EventActor {
    type Context = Context<Self>;
}

#[derive(Clone)]
pub struct EventRpc {
    actor: Arc<Addr<EventActor>>,
}

impl Rpc for EventRpc {
    fn boxing(&self) -> Pin<Box<dyn Rpc + Send + 'static>> {
        return Box::pin(EventRpc{
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

pub fn connect_to_event(
) -> (EventRpc, Arc<Addr<EventActor>>) {
    let actor  = Arc::new(
        EventActor::new(
        )
        .start(),
    );

    return (EventRpc{ actor: actor.clone() }, actor);
}
