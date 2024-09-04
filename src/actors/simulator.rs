use std::sync::Arc;
use std::fmt;

use actix::prelude::*;
use actix::Addr;

use chrono::Utc;

use crate::helpers::{PgConn, PgPool};
use crate::actors::redis::RedisActor;
use crate::actors::cron::CronResolver;


/* @NOTE: how our simulator works?
 *  - Each player will have limited money
 *  - Money will be reduced over the time, we call it inflation, money will 
 *  keep value but fee for burning will remain to simulate how player suviver
 *  with limited money
 *  - When order happens, players must wait 2.5 working days to sell stock
 *
 *  Which formular will be used?
 *  Decision(x) = Sigmoid(a*x + b)
 *  
 */

#[derive(Debug, Clone)]
struct Player {
}

#[derive(Debug, Clone)]
pub struct SimulatorError {
    message: String
}

impl fmt::Display for SimulatorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct SimulatorActor {
    players: Vec<Player>,
}

impl SimulatorActor {
    fn new(n_player: usize) -> Self {
        Self {
            players: vec![Player{}; n_player],
        }
    }
}

impl Actor for SimulatorActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "Result<i64, SimulatorError>")]
pub struct UpdateSettlementCycleCommand;

impl Handler<UpdateSettlementCycleCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<i64, SimulatorError>>;

    fn handle(&mut self, _msg: UpdateSettlementCycleCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            Ok(0)
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<f64, SimulatorError>")]
pub struct OrderCommand {
    id:     i64,
    symbol: String,
    is_buy: bool,
}

impl Handler<OrderCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<f64, SimulatorError>>;

    fn handle(&mut self, msg: OrderCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            if msg.is_buy {
                // @TODO: kiem tra muc hieu suat mua voi co phieu cu the
            } else {
                // @TODO: kiem tra muc hieu suat ban voi co phieu cu the
            }

            Ok(0.0 as f64)
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct EvaluateFitnessCommand;

impl Handler<EvaluateFitnessCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, _msg: EvaluateFitnessCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            Ok(None) 
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct DoSelectionCommand;

impl Handler<DoSelectionCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, _msg: DoSelectionCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            Ok(None) 
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct DoCrossoverCommand;

impl Handler<DoCrossoverCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, _msg: DoCrossoverCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            Ok(None)
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct DoMutationCommand;

impl Handler<DoMutationCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, _msg: DoMutationCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            Ok(None) 
        })
    }
}

pub fn connect_to_simulator(
    resolver: &mut CronResolver,
    pool:     Arc<PgPool>,
    cache:    Arc<Addr<RedisActor>>, 
    token:    String,
    n_player: usize,
) -> Addr<SimulatorActor> {
    let actor   = SimulatorActor::new(n_player)
        .start();
    let simulator: Arc<Addr<SimulatorActor>> = actor.clone().into();

    review_settlement_cycle(
        resolver,
        pool.clone(),
        cache.clone(),
        simulator.clone(),
    );

    review_and_put_orders(
        resolver,
        pool.clone(),
        cache.clone(),
        simulator.clone(),
        n_player,
    );
    return actor;
}

fn review_settlement_cycle(
    resolver: &mut CronResolver,
    pool:     Arc<PgPool>,
    cache:    Arc<Addr<RedisActor>>, 
    simulator: Arc<Addr<SimulatorActor>>,
) {
    resolver.resolve("simulator.review_settlement_cycle".to_string(), move || {
        let simulator = simulator.clone();

        async move {
            simulator.send(UpdateSettlementCycleCommand)
                .await;
        }
    });
}

fn review_and_put_orders(
    resolver:  &mut CronResolver,
    pool:      Arc<PgPool>,
    cache:     Arc<Addr<RedisActor>>, 
    simulator: Arc<Addr<SimulatorActor>>,
    n_player:  usize,
) {
    resolver.resolve("simulator.review_and_put_orders".to_string(), move || {
        let simulator = simulator.clone();
        let pool      = pool.clone();
        let time      = Utc::now().timestamp();

        async move {
            let decisions = (1..n_player)
                .map(move |id| {
                    simulator.send(OrderCommand{
                        id:     id as i64,
                        symbol: "".to_string(),
                        is_buy: true,
                    })
                })
                .collect::<Vec<_>>();
        }
    });
}
