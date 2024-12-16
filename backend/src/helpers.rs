use juniper::{EmptySubscription, RootNode};
use tokio::sync::broadcast;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use crate::schemas::graphql::{Context, Mutation, Query};

pub type SchemaGraphQL = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;
pub type PgConnMgr = ConnectionManager<PgConnection>;
pub type PgConn = PooledConnection<ConnectionManager<PgConnection>>;
pub type PgPool = Pool<PgConnMgr>;

pub fn connect_to_postgres_pool(pg_dsn: String) -> PgPool {
    // @NOTE: establish connection pool with our database
    PgPool::builder()
        .max_size(2)
        .build(PgConnMgr::new(pg_dsn))
        .unwrap()
}

pub fn create_graphql_schema() -> SchemaGraphQL {
    SchemaGraphQL::new(Query {}, Mutation {}, EmptySubscription::new())
}

#[derive(Debug)]
pub struct Shutdown {
    /// `true` if the shutdown signal has been received
    is_shutdown: bool,

    /// The receive half of the channel used to listen for shutdown.
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub(crate) fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            is_shutdown: false,
            notify,
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub(crate) async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.is_shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.is_shutdown = true;
    }
}