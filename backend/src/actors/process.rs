use std::error;
use std::fmt;
use std::io::BufReader;
use std::io::Read;
use std::sync::Arc;
use std::process::{Command, Child, Stdio};

use log::{info, debug, error};

use sentry::capture_error;

use actix::prelude::*;
use actix::Addr;

use crate::schemas::database::tbl_processes::arguments;

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
    commands: Vec<String>,
    arguments: Vec<Vec<String>>,
}

impl ProcessActor {
    fn new() -> Self {
        Self {
            processes: Vec::<Child>::new(),
            commands: Vec::<String>::new(),
            arguments: Vec::<Vec<String>>::new(),
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
        let mut status = true;

        for mut child in &mut self.processes {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        error!("Process {} failed with status {}", child.id(), status);
                    } else {
                        info!("Proces {} exited successfully", child.id());
                    }
                }
                Ok(None) => {
                }
                Err(err) => {
                    capture_error(&err);
                    error!("Error checking process fails: {}", err); 

                    return Box::pin(async move { false });
                }
            }
        }

        return Box::pin(async move { status });
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
        let command = msg.command.clone();
        let args = msg.arguments.clone();
        let result = Command::new(msg.command)
            .args(msg.arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match result {
            Ok(child) => {
                self.processes.push(child);
                self.commands.push(command.clone());
                self.arguments.push(args.clone());

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

pub fn connect_to_process_manager() -> Arc<Addr<ProcessActor>> {
    Arc::new(ProcessActor::new().start())
}
