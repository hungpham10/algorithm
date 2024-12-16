use std::sync::Arc;

use crate::actors::cron::{CronActor, ScheduleCommand};
use crate::actors::process::{ProcessActor, RunCommand};
use crate::helpers::PgPool;

use actix::Addr;
use diesel::prelude::*;

#[derive(Queryable, Clone)]
struct Cron {
    id: i32,
    timeout: i32,
    interval: String,
    resolver: String,
}
#[derive(Queryable, Clone)]
struct Process {
    id: i32,
    instance: String,
    command: String,
    arguments: String,
}

pub async fn load_and_map_schedulers_with_resolvers(pool: Arc<PgPool>, scheduler: Arc<Addr<CronActor>>) {
    use crate::schemas::database::tbl_crons::dsl::*;

    let mut dbconn = pool.get().unwrap();
    let crons = tbl_crons.limit(10).load::<Cron>(&mut dbconn).unwrap();

    for cron in crons {
        let _ = scheduler
            .send(ScheduleCommand {
                cron: cron.interval,
                timeout: cron.timeout,
                route: cron.resolver,
            })
            .await
            .unwrap();
    }
}

pub async fn load_sub_processes_from_pgpool(pool: Arc<PgPool>, target: String, manager: Arc<Addr<ProcessActor>>) {
    use crate::schemas::database::tbl_processes::dsl::*;

    let mut dbconn = pool.get().unwrap();
    let processes = tbl_processes
        .filter(instance.eq(target))
        .limit(10)
        .load::<Process>(&mut dbconn)
        .unwrap();
    for process in processes {
        let _ = manager.send(RunCommand{ 
            command: process.command, 
            arguments: process.arguments.split(' ').map(String::from).collect(),
        })
        .await
        .unwrap();
    }
}