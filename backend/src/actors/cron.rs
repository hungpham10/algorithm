use std::boxed::Box;
use std::clone::Clone;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};

use actix::prelude::*;
use actix::Addr;

use crate::algorithm::heap::Heap;

#[derive(Debug, Clone)]
pub struct Task {
    id:       i64,
    timer:    i64,
    timeout:  i32,
    route:    String,
    interval: String,
    mapping:  BTreeMap<String, String>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

type AsyncCallback =
    Box<dyn Fn(BTreeMap<String, String>, i32, i32) -> Pin<Box<dyn Future<Output = ()>>>>;

pub struct CronResolver {
    resolvers: BTreeMap<String, AsyncCallback>,
}

impl CronResolver {
    pub fn new() -> Self {
        CronResolver {
            resolvers: BTreeMap::new(),
        }
    }

    pub fn commands(&self) -> Vec<String> {
        self.resolvers.keys().map(|k| k.to_string()).collect()
    }

    pub async fn perform(
        &self,
        routes: Vec<String>,
        timeouts: Vec<i32>,
        arguments: Vec<BTreeMap<String, String>>,
        from: i32,
        to: i32,
    ) -> usize {
        let mut cnt = routes.len();

        for (i, route) in routes.iter().enumerate() {
            match self.resolvers.get(route) {
                Some(callback) => {
                    tokio::time::timeout(
                        Duration::from_secs(timeouts[i] as u64),
                        callback(arguments[i].clone(), from, to),
                    )
                    .await
                    .ok();
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
        C: Fn(BTreeMap<String, String>, i32, i32) -> F,
        C: 'static,
        F: Future<Output = ()> + 'static,
    {
        self.resolvers.insert(
            route.clone(),
            Box::new(move |arguments, from, to| Box::pin(callback(arguments, from, to))),
        );
    }
}

pub struct CronActor {
    // @NOTE: shared parameters
    timekeeper: Heap<Task>,
    tick: AtomicI64,
    clock: i64,

    // @NOTE: dependencies
    resolver: Arc<CronResolver>,
}

impl CronActor {
    fn new(resolver: Arc<CronResolver>) -> Self {
        CronActor {
            timekeeper: Heap::<Task>::new(|l: &Task, r: &Task| -> i64 {
                if r.timer == l.timer {
                    return r.id - l.id;
                }

                return r.timer - l.timer;
            }),
            clock: Utc::now().timestamp(),
            tick: AtomicI64::new(1),
            resolver: resolver,
        }
    }

    pub fn commands(&self) -> Vec<String> {
        self.resolver.commands()
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
        let mut timeouts = Vec::<i32>::new();
        let mut mappings = Vec::<BTreeMap<String, String>>::new();
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
                timeouts.push(target.timeout.clone());
                mappings.push(target.mapping.clone());
            }
        }

        self.clock = clock_now.timestamp();

        return Box::pin(async move {
            Ok(resolver
                .perform(targets, timeouts, mappings, -1, -1)
                .await)
        });
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<usize, CronError>")]
pub struct PerformCommand {
    pub target: String,
    pub timeout: i32,
    pub mapping: BTreeMap<String, String>,
    pub from: i32,
    pub to: i32,
}

impl Handler<PerformCommand> for CronActor {
    type Result = ResponseFuture<Result<usize, CronError>>;

    fn handle(&mut self, msg: PerformCommand, _: &mut Self::Context) -> Self::Result {
        let target = vec![msg.target];
        let timeout = vec![msg.timeout];
        let resolver = self.resolver.clone();

        return Box::pin(async move {
            Ok(resolver
                .perform(target, timeout, vec![msg.mapping], msg.from, msg.to)
                .await)
        });
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<i64, CronError>")]
pub struct ScheduleCommand {
    pub cron:    String,
    pub timeout: i32,
    pub route:   String,
    pub mapping: BTreeMap<String, String>,
}

impl Handler<ScheduleCommand> for CronActor {
    type Result = ResponseFuture<Result<i64, CronError>>;

    fn handle(&mut self, msg: ScheduleCommand, _: &mut Self::Context) -> Self::Result {
        let id = self.timekeeper.size();
        let now = Utc::now();

        if let Ok(next) = cron_parser::parse(msg.cron.as_str(), &now) {
            self.timekeeper.push(Task {
                id:       id as i64,
                timeout:  msg.timeout,
                timer:    next.timestamp() - now.timestamp(),
                route:    msg.route.clone(),
                interval: msg.cron.clone(),
                mapping:  msg.mapping.clone(),
            });

            return Box::pin(async move { Ok(id as i64) });
        }

        return Box::pin(async move { Ok(0) });
    }
}

impl Handler<super::HealthCommand> for CronActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
        let tick = *self.tick.get_mut();

        Box::pin(async move { tick > 0 })
    }
}

pub fn connect_to_cron(resolver: Arc<CronResolver>) -> Addr<CronActor> {
    CronActor::new(resolver).start()
}
