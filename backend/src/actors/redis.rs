use redis::{Client, aio::MultiplexedConnection};

use actix::prelude::*;
use actix::Addr;

use crate::components::simulator;

pub struct RedisActor {
    conn: MultiplexedConnection,
}

impl RedisActor {
    async fn new(redis_dsn: String) -> Self {
        RedisActor{
            conn: Client::open(redis_dsn.as_str())
                    .unwrap()
                    .get_multiplexed_async_connection()
                    .await
                    .unwrap(),
        }
    }
}

impl Actor for RedisActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<String>, redis::RedisError>")]
pub struct InfoCommand;

impl Handler<InfoCommand> for RedisActor {
    type Result = ResponseFuture<Result<Option<String>, redis::RedisError>>;

    fn handle(&mut self, _msg: InfoCommand, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.conn.clone();
        let cmd = redis::cmd("INFO");

        Box::pin(async move {
            cmd.query_async(&mut conn)
                .await
        })
    }
}

// ---------------------------------------------------------------------------- //
// @NOTE: action for simulator
//
// ---------------------------------------------------------------------------- //

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<String>, redis::RedisError>")]
pub struct StoreSimulatorCommand{
    pub stock:      String,
    pub session_id: i64,
    pub properties: Vec<simulator::Arguments>,
}

impl Handler<StoreSimulatorCommand> for RedisActor {
    type Result = ResponseFuture<Result<Option<String>, redis::RedisError>>;

    fn handle(&mut self, msg: StoreSimulatorCommand, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.conn.clone();
        let mut cmd = redis::cmd("HSET");
        let stock = msg.stock.clone();
        let session_id = msg.session_id.clone();
        let properties = msg.properties.clone();

        Box::pin(async move {
            cmd.arg(format!("simulator:{}:{}", stock, session_id))
                .arg("properties")
                .arg(serde_json::to_string(&properties).unwrap())
                .query_async(&mut conn)
                .await
        })
    }
}

// ---------------------------------------------------------------------------- //
// @NOTE: end
// ---------------------------------------------------------------------------- //

pub async fn connect_to_redis(redis_dsn: String) -> Addr<RedisActor> {
    RedisActor::new(redis_dsn)
        .await
        .start()
}
