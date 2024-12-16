use std::sync::Arc;
use std::fmt;
use actix::prelude::*;
use actix::Addr;
use rand::Rng;
use chrono::Utc;

use crate::algorithm::genetic::{Genetic, Player};
use crate::helpers::PgPool;
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
struct SimulatorContext {
    datasource: Arc<Vec<Candle>>,
    lookback_order_history: usize,
    lookback_candle_history: usize, 
    batch_money_for_fund: usize,
    arg_gen_min: f64, 
    arg_gen_max: f64,
    money: f64,
    orders: Arc<Vec<f64>>,
}
#[derive(Debug, Clone)]
struct Investor {
    context: Arc<SimulatorContext>,
    market_arguments: Vec<f64>,
    risk_arguments: Vec<f64>,
    fund: f64,
    cache: Arc<Addr<RedisActor>>,
}

impl Investor { 
    fn new(
        context: Arc<SimulatorContext>,
        cache: Arc<Addr<RedisActor>>,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let lookback_order_history = &context.lookback_order_history;
        let lookback_candle_history = &context.lookback_candle_history;
        let batch_money_for_fund = &context.batch_money_for_fund;
        let arg_gen_min = &context.arg_gen_min;
        let arg_gen_max = &context.arg_gen_max;
        let money = &context.money;

        Self {
            context: context.clone(),
            market_arguments: (0..(5 * (*lookback_candle_history))).map(|_| rng.gen_range(*arg_gen_min..*arg_gen_max)).collect(),
            risk_arguments: (0..(*lookback_order_history)).map(|_| rng.gen::<f64>()).collect(),
            fund: (*money) / ((*batch_money_for_fund) as f64),
            cache: cache.clone(),
        }
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    fn tanh(x: f64) -> f64 {
        x.tanh()
    }

    fn merge_using_random_picking_argument_base_on_dominance(
        father_obj: &Investor, father_assets: f64,
        mother_obj: &Investor, mother_assets: f64,
        rng: &mut impl Rng,
    ) -> Self {
        let mut market_arguments = vec![0.0; father_obj.market_arguments.len()];
        let mut risk_arguments = vec![0.0; father_obj.risk_arguments.len()];
        let dominance = father_assets / mother_assets;

        // @NOTE: random picks market arguments base on dominance indicator
        for i in 0..market_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                market_arguments[i] = father_obj.market_arguments[i];
            } else {
                market_arguments[i] = mother_obj.market_arguments[i];
            }
        }

        // @NOTE: random picks risk arguments base on dominance indicator
        for i in 0..risk_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                risk_arguments[i] = father_obj.risk_arguments[i];
            } else {
                risk_arguments[i] = mother_obj.risk_arguments[i];
            }
        }

        Self {
            context: father_obj.context.clone(),
            market_arguments: market_arguments,
            risk_arguments: risk_arguments,
            fund: (father_obj.fund + mother_obj.fund)/2.0,
            cache: father_obj.cache.clone(),
        }
    }
}

impl Player for Investor { 
    fn initialize(&mut self) {
        // @TODO: pull metadata of each investor from redis
        
    }
   
    fn estimate(&self) -> f64 {
        let mut money = self.context.money;
        let mut stock = 0.0;
        let mut scroll = 0 as usize;
        let mut orders = (*self.context.orders).clone();

        for i in 0..(self.context.datasource.len() - self.market_arguments.len()/5) {
            let mut count_selling_order = 0;
            let mut count_buying_order = 0;
            let mut reward = 0.0;
            let mut risk = 0.0;
            let mut j = 0 as usize;

            // @NOTE: estimate market flow using market arguments to adapt and follow candles
            for k in 0..self.market_arguments.len()/5 {
                reward += self.market_arguments[5*k + 0] * self.context.datasource[k + i].open +
                     self.market_arguments[5*k + 1] * self.context.datasource[k + i].high +
                     self.market_arguments[5*k + 2] * self.context.datasource[k + i].close +
                     self.market_arguments[5*k + 3] * self.context.datasource[k + i].low +
                     self.market_arguments[5*k + 4] * self.context.datasource[k + i].volume;
            }

            // @NOTE: count number kind of orders
            for order in &orders {
                if *order < 0.0 {
                    count_selling_order += 1;
                } else {
                    count_buying_order += 1;
                }
            }

            // @NOTE: how manage risk and reward using risk arguments to adjust during orders
            for order in orders.iter().rev() {
                if j >= count_buying_order - count_selling_order {
                    break;
                }

                if *order > 0.0 {
                    risk += self.risk_arguments[j] * (*order);
                    j += 1;
                }
            } 

            // @NOTE: formular to calculate money and stock
            let decison = Investor::tanh(reward) * Investor::sigmoid(risk);
            let fund_stock = self.fund / self.context.datasource[i].close;
            let fund_money = self.fund;

            if decison > 0.9 && money > fund_money {
                orders.push(self.context.datasource[i].close);
                stock += fund_stock;
                money -= fund_money;
            } else if decison < 0.9 && stock > self.fund / self.context.datasource[i].close {
                orders.push(-self.context.datasource[i].close);
                stock -= fund_stock;
                money += fund_money;
            }

        }

        // @TODO: push investors metadata to redis if needs

        return money + stock*self.context.datasource[self.context.datasource.len() - 1].close;
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
    controler: Genetic<Investor>,
    datasource: Arc<Vec<Candle>>,
}

impl SimulatorActor {
    fn new(
        n_player: usize, 
        lookback_history: usize, 
        lookback_order: usize,
        money: f64, 
        min: f64, max: f64,
        candles: Vec<Candle>,
        cache: Arc<Addr<RedisActor>>,
        pool: Arc<PgPool>,
    ) -> Self {
        let datasource = Arc::new(candles);
        let context = Arc::new(SimulatorContext{
            datasource: datasource.clone(),
            lookback_candle_history: lookback_history, 
            lookback_order_history: lookback_order, 
            money: money,
            orders: Arc::new(Vec::<f64>::new()),
            batch_money_for_fund: 100, 
            arg_gen_min: min, 
            arg_gen_max: max,
        });

        Self {
            datasource: datasource.clone(),
            controler: Genetic::<Investor>::new(
                (0..n_player).map(|_| Investor::new(context.clone(), cache.clone())).collect(),
                n_player,
                Self::crossover,
            ),
        }
    }

    fn crossover(
        controler: &Genetic<Investor>,
        father_ctx: &Investor, father_id: usize, 
        mother_ctx: &Investor, mother_id: usize,
        session_id: i64,
    ) -> Investor {
        let mut rng = rand::thread_rng();
        let father_assets = controler.get(father_id).estimate(session_id);
        let mother_assets = controler.get(mother_id).estimate(session_id);

        Investor::merge_using_random_picking_argument_base_on_dominance(
            father_ctx, father_assets,
            mother_ctx, mother_assets,
            &mut rng,
        )
    }
}

impl Actor for SimulatorActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct EvaluateFitnessCommand {
    number_of_couple: usize,
    session: i64,
}

impl Handler<EvaluateFitnessCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, msg: EvaluateFitnessCommand, _: &mut Self::Context) -> Self::Result {
        self.controler.evolute(msg.number_of_couple, msg.session);

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
    n_player: usize,
    n_children: usize,
    candles:  Vec<Candle>,
) -> Addr<SimulatorActor> {
    let actor   = SimulatorActor::new(
            n_player,
            200,
            10,
            200_000_000.0,
            -100.0,
            100.0,
            candles,
            cache.clone(),
            pool.clone(),
        ).start();
    let simulator: Arc<Addr<SimulatorActor>> = actor.clone().into();

    train_investors(
        resolver,
        simulator.clone(),
        n_children,
    );

    return actor;
}

fn train_investors(
    resolver: &mut CronResolver,
    simulator: Arc<Addr<SimulatorActor>>,
    number_of_couple: usize,
) {
    resolver.resolve("simulator.perform_training_investors".to_string(), 
        move |arguments, _, _| {
            let simulator = simulator.clone();
            let session = arguments.get("session")
                .map_or(0, |session| (*session).parse::<i64>().unwrap());

            async move {
                let _ = simulator.send(
                    EvaluateFitnessCommand{
                        number_of_couple: number_of_couple,
                        session: session,
                    },
                )
                .await;
            }
        }
    );
}

