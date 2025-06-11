use std::boxed::Box;
use std::clone::Clone;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use chrono::{TimeZone, Utc};
use futures::future::join_all;

#[cfg(feature = "python")]
use std::sync::Arc;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::PyDict;

use actix::prelude::*;
use actix::Addr;

use crate::algorithm::heap::Heap;

#[derive(Debug, Clone)]
pub struct Task {
    // @NOTE:
    id: i64,
    timer: i64,
    timeout: i32,
    route: String,
    interval: String,

    // @NOTE:
    jsfuzzy: Option<String>,

    #[cfg(feature = "python")]
    pyfuzzy: Option<Arc<Py<PyDict>>>,

    #[cfg(feature = "python")]
    pycallback: Option<Arc<Py<PyAny>>>,
}

impl Task {
    #[cfg(feature = "python")]
    pub fn pycallback(&self) -> Option<Arc<Py<PyAny>>> {
        self.pycallback.clone()
    }

    #[cfg(feature = "python")]
    pub fn pyfuzzy(&self) -> Option<Arc<Py<PyDict>>> {
        self.pyfuzzy.clone()
    }

    pub fn jsfuzzy(&self) -> Option<String> {
        self.jsfuzzy.clone()
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

type AsyncCallback = Box<dyn Fn(Task, i32, i32) -> Pin<Box<dyn Future<Output = ()>>>>;

pub struct CronResolver {
    resolvers: BTreeMap<String, AsyncCallback>,
}

impl Default for CronResolver {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn size(&self) -> usize {
        self.resolvers.len()
    }

    pub async fn perform(&self, tasks: Vec<Task>, from: i32, to: i32) -> usize {
        let mut concurrents = Vec::new();
        let mut cnt = tasks.len();

        for task in tasks {
            if let Some(callback) = self.resolvers.get(&task.route) {
                concurrents.push(tokio::time::timeout(
                    Duration::from_secs(task.timeout as u64),
                    callback(task.clone(), from, to),
                ));
            } else {
                cnt -= 1;
            }
        }

        for result in join_all(concurrents).await {
            match result {
                Ok(_) => {}
                Err(_) => {
                    cnt -= 1;
                }
            }
        }
        cnt
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
    resolver: Rc<CronResolver>,
}

impl CronActor {
    fn new(resolver: Rc<CronResolver>) -> Self {
        CronActor {
            timekeeper: Heap::<Task>::new(|l: &Task, r: &Task| -> i64 {
                if r.timer == l.timer {
                    r.id - l.id
                } else {
                    r.timer - l.timer
                }
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
    message: String,
}

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
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
            // @NOTE: happen when timer run too fast and reach this point
            return Box::pin(async move { Ok(0) });
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

        Box::pin(async move {
            let expected = tasks.len();

            if expected > 0 {
                let ret = resolver.perform(tasks, -1, -1).await;

                if ret == expected {
                    Ok(ret)
                } else {
                    Err(CronError {
                        message: format!(
                            "Fail trigger tasks [actual({}) != expected({})]",
                            ret, expected,
                        ),
                    })
                }
            } else {
                Ok(0)
            }
        })
    }
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "i64")]
pub struct ScheduleCommand {
    pub cron: String,
    pub timeout: i32,
    pub route: String,
    pub jsfuzzy: Option<String>,

    #[cfg(feature = "python")]
    pub pyfuzzy: Option<Arc<Py<PyDict>>>,

    #[cfg(feature = "python")]
    pub pycallback: Option<Arc<Py<PyAny>>>,
}

impl Handler<ScheduleCommand> for CronActor {
    type Result = ResponseFuture<i64>;

    fn handle(&mut self, msg: ScheduleCommand, _: &mut Self::Context) -> Self::Result {
        let id = self.timekeeper.size();
        let now = Utc::now();

        if let Ok(next) = cron_parser::parse(msg.cron.as_str(), &now) {
            self.timekeeper.push(Task {
                id: id as i64,
                timeout: msg.timeout,
                timer: next.timestamp() - now.timestamp(),
                route: msg.route.clone(),
                interval: msg.cron.clone(),
                jsfuzzy: msg.jsfuzzy.clone(),

                #[cfg(feature = "python")]
                pyfuzzy: msg.pyfuzzy.clone(),

                #[cfg(feature = "python")]
                pycallback: msg.pycallback.clone(),
            });

            Box::pin(async move { id as i64 })
        } else {
            Box::pin(async move { 0 })
        }
    }
}

impl Handler<super::HealthCommand> for CronActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
        let tick = *self.tick.get_mut();

        Box::pin(async move { tick > 0 })
    }
}

pub fn connect_to_cron(resolver: Rc<CronResolver>) -> Addr<CronActor> {
    CronActor::new(resolver).start()
}
