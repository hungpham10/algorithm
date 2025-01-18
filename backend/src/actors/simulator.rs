use std::sync::Arc;
use std::fmt;
use std::simd::f64x8;
use std::simd::prelude::SimdFloat;
use std::sync::atomic::{AtomicUsize, Ordering};

use actix::prelude::*;
use actix::Addr;
use rand::Rng;
use rand_distr::{Normal, Distribution};
use serde::{Deserialize, Serialize};

use crate::algorithm::genetic::{Genetic, Player};
use crate::actors::redis::RedisActor;
use crate::actors::dnse::{DnseActor, GetOHCLCommand};
use crate::actors::cron::CronResolver;
use crate::schemas::CandleStick;

/** @NOTE: ideal of setup simulator and use
 * Method to resolve this problem
 *  GA = Genetic<Investor>
 *  Investor ~= Row in ndarray
 *  Vec<Investor> ~= Matrix
 * 
 * Description of each parameter
 *  Investor = (market_arguments, risk_arguments, fund)
 *  market_arguments = state of the art of market stock
 *  risk_arguments = feeling about risk and reward for each investor
 *  fund = money of each investor
 * 
 * Thinking about logic of this simulator:
 *  - when market raise, investor who has more stock in low price will become
 *  better (low risk, high reward)
 *  - when market down, investor who has more money will become better (low 
 *  risk, high reward)
 */


#[derive(Clone, Debug)]
struct Setting {
    candles: Arc<Vec<f64>>,
    number_of_candle: usize,
    lookback_order_history: usize,
    lookback_candle_history: usize, 
    batch_money_for_fund: usize,
    arg_gen_min: f64, 
    arg_gen_max: f64,
    money: f64,
    orders: Arc<Vec<f64>>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Arguments {
    market_arguments: Vec<f64>,
    risk_order_arguments: Vec<f64>,
    risk_market_arguments: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct Investor {
    context: Arc<Setting>,
    fund: f64,
    cache: Arc<Addr<RedisActor>>,
    arguments: Arguments,
}

impl Investor { 
    fn new(
        context: Arc<Setting>,
        cache: Arc<Addr<RedisActor>>,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let lookback_order_history = &context.lookback_order_history;
        let lookback_candle_history = &context.lookback_candle_history;
        let max_market_arguments = 5 * (*lookback_candle_history); 
        let batch_money_for_fund = &context.batch_money_for_fund;
        let arg_gen_min = &context.arg_gen_min;
        let arg_gen_max = &context.arg_gen_max;
        let money = &context.money;

        let mut market_arguments: Vec<f64> = (0..max_market_arguments).map(|_| rng.gen_range(*arg_gen_min..*arg_gen_max)).collect();
        let risk_market_arguments = (0..(*lookback_candle_history)).map(|_| rng.gen::<f64>()).collect();
        let risk_order_arguments = (0..(*lookback_order_history)).map(|_| rng.gen::<f64>()).collect();

        if max_market_arguments % 8 != 0 {
            market_arguments.resize(max_market_arguments + 8 - max_market_arguments % 8, 0.0);
        }

        Self {
            context: context.clone(),
            arguments: Arguments {
                market_arguments,
                risk_order_arguments,
                risk_market_arguments,
            },
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
        let mut market_arguments = vec![0.0; father_obj.arguments.market_arguments.len()];
        let mut risk_order_arguments = vec![0.0; father_obj.arguments.risk_order_arguments.len()];
        let mut risk_market_arguments = vec![0.0; father_obj.arguments.risk_market_arguments.len()];
        let dominance = father_assets / mother_assets;

        // @NOTE: random picks market arguments base on dominance indicator
        for i in 0..market_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                market_arguments[i] = father_obj.arguments.market_arguments[i];
            } else {
                market_arguments[i] = mother_obj.arguments.market_arguments[i];
            }
        }

        // @NOTE: random picks risk arguments base on dominance indicator
        for i in 0..risk_order_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                risk_order_arguments[i] = father_obj.arguments.risk_order_arguments[i];
            } else {
                risk_order_arguments[i] = mother_obj.arguments.risk_order_arguments[i];
            }
        }

        for i in 0..risk_market_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                risk_market_arguments[i] = father_obj.arguments.risk_market_arguments[i];
            } else {
                risk_market_arguments[i] = mother_obj.arguments.risk_market_arguments[i];
            }
        }

        Self {
            context: father_obj.context.clone(),
            arguments: Arguments {
                market_arguments: market_arguments,
                risk_order_arguments: risk_order_arguments,
                risk_market_arguments: risk_market_arguments,
            },
            fund: (father_obj.fund + mother_obj.fund)/2.0,
            cache: father_obj.cache.clone(),
        }
    }

    fn perform_stock_order_strategy(
        &self, 
    ) -> (f64, f64, Vec<f64>) {
        let mut money = self.context.money;
        let mut stock = 0.0;
        let mut orders = (*self.context.orders).clone();
        let mut sentiments = vec![0.0; self.arguments.risk_market_arguments.len()];

        for i in 0..(self.context.number_of_candle - self.context.lookback_candle_history) {
            let mut count_selling_order = 0;
            let mut count_buying_order = 0;
            let mut indicator = 0.0;
            let mut risk = 0.0;
            let k_limit = self.arguments.market_arguments.len()/8;


            // @NOTE: estimate market flow using market arguments to adapt and follow candles
            for k in 0..k_limit {
                let market_arguments = f64x8::from_slice(
                    &self.arguments.market_arguments[8*k..8*k+8],
                );
                let candle = f64x8::from_slice(
                    &self.context.candles[8*k + 5*i..8*k + 5*i + 8],
                );
                let mult = market_arguments * candle;

                indicator += mult.reduce_sum();

                //indicator += self.market_arguments[5*k + 0] * self.context.candles[k + i].o +
                //     self.market_arguments[5*k + 1] * self.context.candles[k + i].h +
                //     self.market_arguments[5*k + 2] * self.context.candles[k + i].c +
                //     self.market_arguments[5*k + 3] * self.context.candles[k + i].l +
                //     self.market_arguments[5*k + 4] * self.context.candles[k + i].v as f64 / volume_calibrate;
            }

            // @NOTE: count number kind of orders
            for order in &orders {
                if *order < 0.0 {
                    count_selling_order += 1;
                } else {
                    count_buying_order += 1;
                }
            }

            // @NOTE: how manage risk and indicator using risk arguments to adjust during orders
            if count_selling_order < count_buying_order {
                let mut j = 0 as usize;

                for order in orders.iter().rev() {
                    if j >= count_buying_order - count_selling_order {
                        break;
                    }

                    if *order > 0.0 && j < self.arguments.risk_order_arguments.len() {
                        risk += self.arguments.risk_order_arguments[j] * (*order);
                        j += 1;
                    }
                }
            }

            // @NOTE: remove the oldest sentiment
            for i in 1..self.arguments.risk_market_arguments.len() {
                sentiments[i - 1] = sentiments[i];
            }

            match sentiments.last_mut() {
                Some(sentiment) => {
                    *sentiment = indicator;
                }
                None => {
                    panic!("last sentiment not found");
                }
            }

            // @NOTE: calculate sentiment
            for i in 0..self.arguments.risk_market_arguments.len() {
                risk += sentiments[i] * self.arguments.risk_market_arguments[i];
            }

            // @NOTE: formular to calculate money and stock
            let decison = (Investor::tanh(indicator) + Investor::sigmoid(risk))/2.0;
            let fund_stock = self.fund / self.context.candles[5*i + 2];
            let fund_money = self.fund;

            // @TODO: implement T+3
            if decison > 0.9 && money > fund_money {
                orders.push(self.context.candles[5*i+2]);
                stock += fund_money / self.context.candles[5*i + 2];
                money -= fund_money;
            } else if decison < 0.9 && stock > self.fund / self.context.candles[5*i + 2] {
                orders.push(-self.context.candles[5*i + 2]);
                stock -= fund_stock;
                money += fund_stock * self.context.candles[5*i + 2];
            }
        }

        return (money, stock, orders.clone());
    }
}

impl Player for Investor { 
    fn initialize(&mut self) {
        let mut rng = rand::thread_rng(); 
   
        for i in 0..self.arguments.market_arguments.len() {
            self.arguments.market_arguments[i] = rng.gen_range(
                self.context.arg_gen_min..self.context.arg_gen_max
            );
        }
        for i in 0..self.arguments.risk_order_arguments.len() {
            self.arguments.risk_order_arguments[i] = rng.gen_range(
                self.context.arg_gen_min..self.context.arg_gen_max
            );
        }
        for i in 0..self.arguments.risk_market_arguments.len() {
            self.arguments.risk_market_arguments[i] = rng.gen_range(
                self.context.arg_gen_min..self.context.arg_gen_max
            );
        }

        // @TODO: pull metadata of each investor from redis        
    }

    fn estimate(&self) -> f64 {
        let (money, _, _) = self.perform_stock_order_strategy();

        // @NOTE: adjust genetic traning flow after each loop to improve performance
        return money;
    }
 
    fn gene(&self) -> Vec<f64> {
        let mut ret = Vec::new();
        
        ret.extend(self.arguments.market_arguments.clone());
        ret.extend(self.arguments.risk_order_arguments.clone());
        ret.extend(self.arguments.risk_market_arguments.clone());

        return ret;
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

struct Simulator {
    controller: Genetic<Investor>,
    stock:      String,
    session:    i64,
}

pub struct SimulatorActor {
    // @NOTE: simulators
    simulators: Vec<Simulator>,
    cache:      Arc<Addr<RedisActor>>,
    loop_id:    AtomicUsize,

    // @NOTE: shared parameters
    lookback_order_history:  usize,
    lookback_candle_history: usize, 
    arg_gen_min:             f64, 
    arg_gen_max:             f64,
    number_of_player:        usize,
}

impl SimulatorActor {
    fn new(
        n_player: usize, 
        lookback_history: usize, 
        lookback_order: usize,
        min: f64, max: f64,
        cache: Arc<Addr<RedisActor>>,
    ) -> Self {
        Self {
            // @NOTE: simulators
            simulators: Vec::new(),

            // @NOTE: cache server
            cache: cache,

            // @NOTE: simulator id
            loop_id: AtomicUsize::new(0),

            // @NOTE; template
            lookback_order_history:  lookback_order,
            lookback_candle_history: lookback_history, 
            arg_gen_min:             min, 
            arg_gen_max:             max,
            number_of_player:        n_player,
        }
    }

    fn prepare_sessions(&mut self, simulator_id: usize, number_of_session: usize) -> i64 {
        let first_session = self.simulators[simulator_id].session;

        self.simulators[simulator_id].session += number_of_session as i64;
        return first_session;
    }

    fn crossover(
        controller: &Genetic<Investor>,
        father_ctx: &Investor, father_id: usize, 
        mother_ctx: &Investor, mother_id: usize,
        session_id: i64,
    ) -> Investor {
        let mut rng = rand::thread_rng();
        let father_assets = controller.get(father_id).estimate(session_id);
        let mother_assets = controller.get(mother_id).estimate(session_id);

        Investor::merge_using_random_picking_argument_base_on_dominance(
            father_ctx, father_assets,
            mother_ctx, mother_assets,
            &mut rng,
        )
    }

    fn policy(_investor: &Investor) -> bool {
        // @TODO: implement policy to remove investor who unable to buy or sell
        //        any more

        return false;
    }

    fn mutate(
        investor: &mut Investor,
        gene: usize,
    ) {
        let mut rng = rand::thread_rng();
        let std_dev = 0.5;
        let sampling = 0.005 * Normal::new(0.0, std_dev).unwrap().sample(&mut rng);

        if gene < investor.arguments.market_arguments.len() {
            investor.arguments.market_arguments[gene] = sampling;
        } else if gene < investor.arguments.risk_order_arguments.len() + investor.arguments.market_arguments.len() {
            investor.arguments.risk_order_arguments[gene - investor.arguments.market_arguments.len()] = sampling;
        } else {
            investor.arguments.risk_market_arguments[
                gene - investor.arguments.market_arguments.len() - investor.arguments.risk_order_arguments.len()
            ] = sampling;
        }
    }
}

impl Actor for SimulatorActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<usize>, SimulatorError>")]
pub struct SetupSettingCommand {
    batch_money_for_fund: usize,
    candles: Arc<Vec<CandleStick>>,
    stock: String,
    money: f64,
}

impl Handler<SetupSettingCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<usize>, SimulatorError>>;

    fn handle(&mut self, msg: SetupSettingCommand, _: &mut Self::Context) -> Self::Result {
        let mut candles = Vec::new();
        let id = self.simulators.len();


        let mut volume_calibrate = msg.candles[0].v as f64;
        
        for i in 0..msg.candles.len() {
            if volume_calibrate > msg.candles[i].v as f64 {
                volume_calibrate = msg.candles[i].v as f64;
            }
        }
        volume_calibrate /= 3.0;

        msg.candles.iter().for_each(|candle| { 
            candles.push(candle.o);
            candles.push(candle.h);
            candles.push(candle.c);
            candles.push(candle.l);
            candles.push(candle.v as f64 / volume_calibrate);
        });

        if candles.len() % 8 != 0 {
            candles.resize(candles.len() + 8 - candles.len() % 8, 0.0);
        }

        let context = Arc::new(Setting{
            candles:                 Arc::new(candles),
            money:                   msg.money,
            number_of_candle:        msg.candles.len(),
            batch_money_for_fund:    msg.batch_money_for_fund, 
            orders:                  Arc::new(Vec::<f64>::new()),
            lookback_candle_history: self.lookback_candle_history,
            lookback_order_history:  self.lookback_order_history, 
            arg_gen_min:             self.arg_gen_min, 
            arg_gen_max:             self.arg_gen_max,
        });
        let cache = self.cache.clone();
        let n_player = self.number_of_player;

        let mut controller = Genetic::<Investor>::new(
            n_player,
            Self::crossover,
            Self::mutate,
            Self::policy,
        );

        controller.initialize(
            (0..n_player)
                .map(|_| Investor::new(context.clone(), cache.clone()))
                .collect(), 
        );

        self.simulators.push(Simulator{
            controller: controller,
            stock:      msg.stock.clone(),
            session:    0,
        });

        Box::pin(async move {
            Ok(Some(id))
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct EvaluateFitnessCommand {
    number_of_couple:    usize,
    number_of_loop:      usize,
    mutation_rate:       f64,
    number_of_simulator: usize,
}

impl Handler<EvaluateFitnessCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, msg: EvaluateFitnessCommand, _: &mut Self::Context) -> Self::Result {
        if self.simulators.len() == 0 {
            return Box::pin(async move {
                Err(SimulatorError{
                    message: "No simulator found".to_string(),
                })
            });
        }

        let cache = self.cache.clone();
        let simulator_ids = (0..msg.number_of_simulator).map(|_| {
                let simulator_id  = self.loop_id.fetch_add(1, Ordering::SeqCst) % self.simulators.len();
                let first_session = self.prepare_sessions(simulator_id, msg.number_of_loop);

                for i in 0..msg.number_of_loop {
                    self.simulators[simulator_id].controller.evolute(
                        msg.number_of_couple, 
                        first_session + i as i64,
                    );
                    self.simulators[simulator_id].controller.fluctuate(
                        first_session + i as i64, 
                        msg.mutation_rate,
                    );
                }

                simulator_id
            })
            .collect::<Vec<usize>>();

        let sessions = simulator_ids.iter()
            .map(|&simulator_id| self.simulators[simulator_id].session)
            .collect::<Vec<i64>>();

        let stocks = simulator_ids.iter()
            .map(|&simulator_id| self.simulators[simulator_id].stock.clone())
            .collect::<Vec<String>>();

        let properties = simulator_ids
            .iter()
            .map(|&simulator_id| {
                (0..self.simulators[simulator_id].controller.size())
                    .map(|i| {
                        self.simulators[simulator_id]
                            .controller
                            .get(i)
                            .into()
                            .arguments
                            .clone()
                    })
                    .collect::<Vec<Arguments>>()
            })
            .collect::<Vec<Vec<Arguments>>>();

        Box::pin(async move {
            for i in 0..simulator_ids.len() {
                cache.send(super::redis::StoreSimulatorCommand {
                        session_id: sessions[i],
                        stock:      stocks[i].clone(),
                        properties: properties[i].clone(),
                    })
                    .await
                    .unwrap()
                    .unwrap_or(None)
                    .is_some();
            }
            Ok(None)
        })
    }
}

pub fn connect_to_simulator(
    resolver:   &mut CronResolver,
    cache:      Arc<Addr<RedisActor>>,
    dnse:       Arc<Addr<DnseActor>>,
    n_player:   usize,
    n_children: usize,
) -> Addr<SimulatorActor> {
    let actor   = SimulatorActor::new(
            n_player,
            50,
            30,
            -0.2,
            0.2,
            cache.clone(),
        ).start();
    let simulator: Arc<Addr<SimulatorActor>> = actor.clone().into();

    setup_environment_for_median_strategy(
        resolver,
        dnse.clone(),
        simulator.clone(),
    );
    train_investors(
        resolver,
        simulator.clone(),
        n_children,
    );

    return actor;
}

fn setup_environment_for_median_strategy(
    resolver: &mut CronResolver,
    dnse: Arc<Addr<DnseActor>>,
    simulator: Arc<Addr<SimulatorActor>>, 
) {
    resolver.resolve("simulator.setup_new_environment_for_median_strategy".to_string(),
        move |arguments, _, _| {
            let simulator = simulator.clone();
            let money = arguments.get("money")
                .map_or(1000000.0, |money| (*money).parse::<f64>().unwrap());
            let dnse = dnse.clone();

            async move {
                for stock in super::vps::list_active_stocks().await {
                    let candles = Arc::new(dnse.send(GetOHCLCommand{
                        resolution: arguments.get("resolution").unwrap().to_string(),
                        stock:      stock.clone(),
                        from:       arguments.get("from").map_or(0, |from| (*from).parse::<i64>().unwrap()),
                        to:         arguments.get("to").map_or(0, |to| (*to).parse::<i64>().unwrap()),
                    })
                    .await
                    .unwrap()
                    .unwrap());

                    let _ = simulator.send(SetupSettingCommand{
                        // @NOTE: median strategy's properties
                        batch_money_for_fund: arguments.get("batch_money_for_fund")
                            .map_or(100, |batch_money_for_fund| (*batch_money_for_fund).parse::<usize>().unwrap()),

                        // @NOTE: commnon properties
                        candles,
                        money,
                        stock: stock.clone(),
                    })
                    .await;
                }
            }
        }
    );
}

fn train_investors(
    resolver: &mut CronResolver,
    simulator: Arc<Addr<SimulatorActor>>,
    number_of_couple: usize,
) {
    resolver.resolve("simulator.perform_training_investors".to_string(), 
        move |arguments, _, _| {
            let simulator = simulator.clone();
            let number_of_simulator = arguments.get("number_of_simulator")
                .map_or(1, |number_of_simulator| (*number_of_simulator).parse::<usize>().unwrap());
            let number_of_loop = arguments.get("number_of_loop")
                .map_or(100, |number_of_loop| (*number_of_loop).parse::<usize>().unwrap());
            let mutation_rate = arguments.get("mutation_rate")
                .map_or(0.1, |mutation_rate| (*mutation_rate).parse::<f64>().unwrap());

            async move {
                let _ = simulator.send(
                    EvaluateFitnessCommand{
                        number_of_couple,
                        number_of_loop,
                        mutation_rate,
                        number_of_simulator,
                    },
                )
                .await;
            }
        }
    );
}

