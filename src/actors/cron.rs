use std::boxed::Box;
use std::clone::Clone;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};

use actix::prelude::*;
use actix::Addr;

use crate::actors::redis::RedisActor;
use crate::algorithm::heap::Heap;
use crate::helpers::PgPool;

#[derive(Debug, Clone)]
pub struct Task {
    id: i64,
    timer: i64,
    route: String,
    interval: String,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

type AsyncCallback = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>>;

pub struct CronResolver {
    resolvers: BTreeMap<String, AsyncCallback>,
}

impl CronResolver {
    pub fn new() -> Self {
        CronResolver {
            resolvers: BTreeMap::new(),
        }
    }

    pub async fn perform(&self, routes: Vec<String>) -> usize {
        let mut cnt = routes.len();

        for route in routes.iter() {
            match self.resolvers.get(route) {
                Some(callback) => {
                    callback().await;
                }
                None => {
                    cnt -= 1;
                }
            }
        }

        return cnt;
    }

    pub fn resolve<C, F>(&mut self, route: String, callback: C)
    where
        C: Fn() -> F,
        C: 'static,
        F: Future<Output = ()> + 'static,
    {
        self.resolvers
            .insert(route.clone(), Box::new(move || Box::pin(callback())));
    }
}

pub struct CronActor {
    // @NOTE: shared parameters
    timekeeper: Heap<Task>,
    tick: AtomicI64,
    clock: i64,

    // @NOTE: dependencies
    resolver: Arc<CronResolver>,
    pool: Arc<PgPool>,
    cache: Arc<Addr<RedisActor>>,
}

impl CronActor {
    fn new(resolver: Arc<CronResolver>, pool: Arc<PgPool>, cache: Arc<Addr<RedisActor>>) -> Self {
        CronActor {
            timekeeper: Heap::<Task>::new(|l: &Task, r: &Task| -> i64 {
                if r.timer == l.timer {
                    return r.id - l.id;
                }

                return r.timer - l.timer;
            }),
            clock: Utc::now().timestamp(),
            tick: AtomicI64::new(1),
            pool: pool,
            cache: cache,
            resolver: resolver,
        }
    }
}

impl Actor for CronActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct CronError {
    code: i32,
}

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.code {
            _ => write!(f, "unknown error"),
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<usize, CronError>")]
pub struct TickCommand;

impl Handler<TickCommand> for CronActor {
    type Result = ResponseFuture<Result<usize, CronError>>;

    fn handle(&mut self, _msg: TickCommand, _: &mut Self::Context) -> Self::Result {
        let mut targets = Vec::<String>::new();
        let clock_now = Utc::now();

        if clock_now.timestamp() == self.clock {
            return Box::pin(async move { Err(CronError { code: 0 }) });
        } else {
        }

        let tick_now = self
            .tick
            .fetch_add(clock_now.timestamp() - self.clock, Ordering::SeqCst);
        let resolver = self.resolver.clone();

        for _ in 0..self.timekeeper.size() {
            let wrapped = self.timekeeper.get_mut();

            if wrapped.is_none() {
                break;
            }

            let plus_1 = Utc.timestamp_opt(clock_now.timestamp() + 1, 0).unwrap();
            let target = wrapped.unwrap().clone();
            if tick_now != target.timer {
                break;
            }

            if let Ok(time_next) = cron_parser::parse(&target.interval, &plus_1) {
                let mut target_next = target.clone();

                target_next.timer = tick_now + (time_next.timestamp() - clock_now.timestamp()) - 1;
                self.timekeeper.push(target_next.clone());
            }

            if self.timekeeper.pop() {
                targets.push(target.route.clone());
            }
        }

        self.clock = clock_now.timestamp();

        return Box::pin(async move { Ok(resolver.perform(targets).await) });
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<i64, CronError>")]
pub struct ScheduleCommand {
    pub cron: String,
    pub route: String,
}

impl Handler<ScheduleCommand> for CronActor {
    type Result = ResponseFuture<Result<i64, CronError>>;

    fn handle(&mut self, msg: ScheduleCommand, _: &mut Self::Context) -> Self::Result {
        let id = self.timekeeper.size();
        let now = Utc::now();

        if let Ok(next) = cron_parser::parse(msg.cron.as_str(), &now) {
            self.timekeeper.push(Task {
                id: id as i64,
                timer: next.timestamp() - now.timestamp(),
                route: msg.route.clone(),
                interval: msg.cron.clone(),
            });

            return Box::pin(async move { Ok(id as i64) });
        }

        return Box::pin(async move { Ok(0) });
    }
}

#[derive(Message, Debug)]
#[rtype(result = "i64")]
pub struct HealthCommand;

impl Handler<HealthCommand> for CronActor {
    type Result = ResponseFuture<i64>;

    fn handle(&mut self, _msg: HealthCommand, _: &mut Self::Context) -> Self::Result {
        let tick = *self.tick.get_mut();

        Box::pin(async move { tick })
    }
}

pub fn connect_to_cron(
    resolver: Arc<CronResolver>,
    pool: Arc<PgPool>,
    cache: Arc<Addr<RedisActor>>,
) -> Addr<CronActor> {
    CronActor::new(resolver, pool, cache).start()
}
