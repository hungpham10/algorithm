use redis::{Client, aio::MultiplexedConnection};

use actix::prelude::*;
use actix::Addr;

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
        let mut con = self.conn.clone();
        let cmd = redis::cmd("INFO");

        Box::pin(async move {
            cmd.query_async(&mut con)
                .await
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<String>, redis::RedisError>")]
pub struct GetCommand;

impl Handler<GetCommand> for RedisActor {
    type Result = ResponseFuture<Result<Option<String>, redis::RedisError>>;

    fn handle(&mut self, _msg: GetCommand, _: &mut Self::Context) -> Self::Result {
        let mut con = self.conn.clone();
        let cmd = redis::cmd("INFO");

        Box::pin(async move {
            cmd.query_async(&mut con)
                .await
        })
    }
}

pub async fn connect_to_redis(redis_dsn: String) -> Addr<RedisActor> {
    RedisActor::new(redis_dsn)
        .await
        .start()
}
