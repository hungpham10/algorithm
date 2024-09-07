use crate::actors::cron::{CronActor, ScheduleCommand};
use crate::helpers::PgPool;
use actix::Addr;
use diesel::prelude::*;

#[derive(Queryable, Clone)]
struct Cron {
    id: i32,
    interval: String,
    resolver: String,
}

pub async fn load_and_map_schedulers_with_resolvers(pool: PgPool, scheduler: Addr<CronActor>) {
    use crate::schemas::database::tbl_crons::dsl::*;

    let mut dbconn = pool.get().unwrap();
    let crons = tbl_crons.limit(10).load::<Cron>(&mut dbconn).unwrap();

    for cron in crons {
        let _ = scheduler
            .send(ScheduleCommand {
                cron: cron.interval,
                route: cron.resolver,
            })
            .await
            .unwrap();
    }
}
