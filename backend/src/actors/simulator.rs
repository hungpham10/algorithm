use std::sync::Arc;
use std::fmt;
use actix::prelude::*;
use actix::Addr;
use rand::Rng;
use chrono::Utc;

use crate::algorithm::genetic::{Genetic, Player};
use crate::helpers::{PgConn, PgPool};
use crate::actors::redis::RedisActor;
use crate::actors::cron::CronResolver;


#[derive(Debug, Clone)]
struct Candle {
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    volume: f64,
}

#[derive(Debug, Clone)]
struct Investor {
    datasource: Arc<Vec<Candle>>,
    arguments: Vec<f64>,
    risks: Vec<f64>,
    history: Vec<f64>,
    stock: f64,
    fund: f64,
    money: f64,
}

impl Investor { 
    fn new(
        lookback_order_history: usize,
        lookback_candle_history: usize, 
        money: f64,
        batch_money_for_fund: usize,
        arg_gen_min: f64, arg_gen_max: f64,
    ) -> Self {
        Self {
            arguments: (0..(5 * lookback_candle_history)).map(|_| rng.gen_range(arg_gen_min..arg_gen_max)).collect(),
            risks: (0..lookback_order_history).map(|_| )
            stock: 0.0,
            fund: money / (batch_money_for_fund as f64),
            money: money,
        }
    }

    fn datasource(&mut self, datasource: Arc<Vec<Candle>>) {
        self.datasource = datasource;
    }

    fn stock(&mut self, stock: f64) {
        self.stock = stock
    }

    fn money(&mut self, money: f64) {
        self.money = money
    }
}

impl Player for Investor { 
    fn initialize(&mut self) {
        /* @NOTE: 
         * f = a[1]*c[0] + a[1]*c[1] + ... + a[n]*c[n]
         * m = m - c[0]*a[0]*tanh(f)
         *
         * a[5*i + 0]*d[i].o
         * a[5*i + 1]*d[i].h
         * a[5*i + 2]*d[i].c
         * a[5*i + 3]*d[i].l
         * a[5*i + 4]*d[i].v
         */
    }

    fn evaluate(&self) -> f64 {
        let mut f = 0.0 as f64;

        // @TODO: do we need a flag to add support calculation with volume or not
        for i in (0..self.arguments.len()).step_by(5) {
            f += self.arguments[5*i + 0] * self.datasource[i].o +
                 self.arguments[5*i + 1] * self.datasource[i].h +
                 self.arguments[5*i + 2] * self.datasource[i].c +
                 self.arguments[5*i + 3] * self.datasource[i].l +
                 self.arguments[5*i + 4] * self.datasource[i].v;
        }

        // @TODO: how manage risk and reward

        // @TODO: decision making
        m -= f.tanh();
        return f;
    }
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
    controler: Arc<Genetic<Investor>>,
}

impl SimulatorActor {
    fn new(n_player: usize, lookback: usize, money: f64, min: f64, max: f64) -> Self {
        Self {
            controler: Arc::new(Genetic::<Investor>::new(
                (0..n_player).map(|_| Investor::new(10, lookback, money, 100, min, max)).collect(),
                n_player,
                Self::crossover,
            )),
        }
    }

    fn crossover(
        controller: &Genetic<Investor>,
        father_ctx: &Investor, father_id: usize, 
        mother_ctx: &Investor, mother_id: usize,
        session_id: i64,
    ) -> Investor {
        let mut rng = rand::thread_rng();
        
        Investor(
        )
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
