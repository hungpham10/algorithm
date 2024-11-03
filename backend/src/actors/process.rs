use std::error;
use std::fmt;
use std::process::{Command, Child, Stdio};

use log::{info, debug, error};

use sentry::capture_error;

use actix::prelude::*;
use actix::Addr;

#[derive(Debug, Clone)]
pub struct ProcessError {
    message: String,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for ProcessError {}

pub struct ProcessActor {
    processes: Vec<Child>,
}

impl ProcessActor {
    fn new() -> Self {
        Self {
            processes: Vec::<Child>::new(),
        }
    }
}

impl Actor for ProcessActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

impl Handler<HealthCommand> for ProcessActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct RunCommand {
    pub command: String,
    pub arguments: Vec<String>,
}

impl Handler<RunCommand> for ProcessActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, msg: RunCommand, _ctx: &mut Self::Context) -> Self::Result {
        let result = Command::new(msg.command)
            .args(msg.arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match result {
            Ok(command) => {
                self.processes.push(command);
                Box::pin(async move { true })
            }
            Err(err) => {
                capture_error(&err);
                error!("Start sub-process fails: {}", err);
                Box::pin(async move { false })
            }
        }
    }
}

pub fn connect_to_process_manager() -> Addr<ProcessActor> {
    ProcessActor::new().start()
}
