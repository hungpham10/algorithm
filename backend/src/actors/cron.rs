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

use pyo3::types::PyDict;
use pyo3::prelude::*;

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

    pyfuzzy:    Arc<Py<PyDict>>,
    pycallback: Arc<Py<PyAny>>,
}

impl Task {
    pub fn pycallback(&self) -> Arc<Py<PyAny>> {
        self.pycallback.clone()
    }

    pub fn pyfuzzy(&self) -> Arc<Py<PyDict>> {
        self.pyfuzzy.clone()
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

type AsyncCallback =
    Box<dyn Fn(Task, i32, i32) -> Pin<Box<dyn Future<Output = ()>>>>;

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
        tasks: Vec<Task>,
        from: i32,
        to: i32,
    ) -> usize {
        let mut cnt = tasks.len();

        for task in tasks {
            match self.resolvers.get(&task.route) {
                Some(callback) => {
                    tokio::time::timeout(
                        Duration::from_secs(task.timeout as u64),
                        callback(task.clone(), from, to),
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
        C: Fn(Task, i32, i32) -> F,
        C: 'static,
        F: Future<Output = ()> + 'static,
    {
        self.resolvers.insert(
            route.clone(),
            Box::new(move |task, from, to| Box::pin(callback(task, from, to))),
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
            resolver,
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
        let mut tasks = Vec::<Task>::new();
        let clock_now = Utc::now();
        
        if clock_now.timestamp() == self.clock {
            return Box::pin(async move { Err(CronError { code: 0 }) });
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
                tasks.push(target); 
            }
        }

        self.clock = clock_now.timestamp();

        return Box::pin(async move {
            Ok(resolver
                .perform(tasks, -1, -1)
                .await)
        });
    }
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<i64, CronError>")]
pub struct ScheduleCommand {
    pub cron:       String,
    pub timeout:    i32,
    pub route:      String,
    pub pyfuzzy:    Arc<Py<PyDict>>,
    pub pycallback: Arc<Py<PyAny>>,
}

impl Handler<ScheduleCommand> for CronActor {
    type Result = ResponseFuture<Result<i64, CronError>>;

    fn handle(&mut self, msg: ScheduleCommand, _: &mut Self::Context) -> Self::Result {
        let id = self.timekeeper.size();
        let now = Utc::now();

        if let Ok(next) = cron_parser::parse(msg.cron.as_str(), &now) {
            self.timekeeper.push(Task {
                id:         id as i64,
                timeout:    msg.timeout,
                timer:      next.timestamp() - now.timestamp(),
                route:      msg.route.clone(),
                interval:   msg.cron.clone(),
                pyfuzzy:    msg.pyfuzzy.clone(),
                pycallback: msg.pycallback.clone(),
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
