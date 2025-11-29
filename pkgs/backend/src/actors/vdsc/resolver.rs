use std::sync::{Arc, Mutex};

use actix::prelude::*;
use actix::Actor;

use crate::actors::cron::CronResolver;
use crate::actors::ActorError;
use crate::algorithm::fuzzy::Variables;

use super::DragonActor;

pub fn resolve_vdsc_routes(
    resolver: &mut CronResolver,
    stocks: &[String],
    variables: Arc<Mutex<Variables>>,
) -> Result<Arc<Addr<DragonActor>>, ActorError> {
    let vdsc = DragonActor::new()?;
    let actor = Arc::new(vdsc.start());

    resolve_watching_vdsc_future_board(actor.clone(), resolver);
    Ok(actor)
}

fn resolve_watching_vdsc_future_board(actor: Arc<Addr<DragonActor>>, resolver: &mut CronResolver) {
    resolver.resolve("vdsc.watch_future_board".to_string(), move |task, _, _| {
        let actor = actor.clone();
        async move {}
    });
}
