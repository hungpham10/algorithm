
use std::sync::Arc;
use std::fmt;
use actix::prelude::*;
use actix::Addr;
use rand::Rng;
use rand_distr::{Normal, Distribution};

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
    candles: Arc<Vec<CandleStick>>,
    lookback_order_history: usize,
    lookback_candle_history: usize, 
    batch_money_for_fund: usize,
    arg_gen_min: f64, 
    arg_gen_max: f64,
    money: f64,
    orders: Arc<Vec<f64>>,
}

#[derive(Clone, Debug)]
pub struct Investor {
    context: Arc<Setting>,
    fund: f64,
    cache: Arc<Addr<RedisActor>>,
    market_arguments: Vec<f64>,
    risk_order_arguments: Vec<f64>,
    risk_market_arguments: Vec<f64>,
}

impl Investor { 
    fn new(
        context: Arc<Setting>,
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
            risk_market_arguments: (0..(*lookback_candle_history)).map(|_| rng.gen::<f64>()).collect(),
            risk_order_arguments: (0..(*lookback_order_history)).map(|_| rng.gen::<f64>()).collect(),
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
        let mut risk_order_arguments = vec![0.0; father_obj.risk_order_arguments.len()];
        let mut risk_market_arguments = vec![0.0; father_obj.risk_market_arguments.len()];
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
        for i in 0..risk_order_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                risk_order_arguments[i] = father_obj.risk_order_arguments[i];
            } else {
                risk_order_arguments[i] = mother_obj.risk_order_arguments[i];
            }
        }

        for i in 0..risk_market_arguments.len() {
            if rng.gen::<f64>() < dominance { 
                risk_market_arguments[i] = father_obj.risk_market_arguments[i];
            } else {
                risk_market_arguments[i] = mother_obj.risk_market_arguments[i];
            }
        }

        Self {
            context: father_obj.context.clone(),
            market_arguments: market_arguments,
            risk_order_arguments: risk_order_arguments,
            risk_market_arguments: risk_market_arguments,
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
        let mut orders = (*self.context.orders).clone();
        let mut sentiments = vec![0.0; self.risk_market_arguments.len()];

        let mut volume_calibrate = self.context.candles[0].v as f64;
        
        for i in 0..self.context.candles.len() {
            if volume_calibrate > self.context.candles[i].v as f64 {
                volume_calibrate = self.context.candles[i].v as f64;
            }
        }
        volume_calibrate /= 3.0;
        
        for i in 0..(self.context.candles.len() - self.market_arguments.len()/5) {
            let mut count_selling_order = 0;
            let mut count_buying_order = 0;
            let mut indicator = 0.0;
            let mut risk = 0.0;
            let mut j = 0 as usize;

            // @NOTE: estimate market flow using market arguments to adapt and follow candles
            for k in 0..self.market_arguments.len()/5 {
                indicator += self.market_arguments[5*k + 0] * self.context.candles[k + i].o +
                     self.market_arguments[5*k + 1] * self.context.candles[k + i].h +
                     self.market_arguments[5*k + 2] * self.context.candles[k + i].c +
                     self.market_arguments[5*k + 3] * self.context.candles[k + i].l +
                     self.market_arguments[5*k + 4] * self.context.candles[k + i].v as f64 / volume_calibrate;
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
            for order in orders.iter().rev() {
                if j >= count_buying_order - count_selling_order {
                    break;
                }

                if *order > 0.0 {
                    risk += self.risk_order_arguments[j] * (*order);
                    j += 1;
                }
            }

            // @NOTE: remove the oldest sentiment
            for i in 1..self.risk_market_arguments.len() {
                sentiments[i - 1] = sentiments[i];
            }

            match sentiments.last_mut() {
                Some(sentiment) => {
                    for k in 0..self.risk_market_arguments.len() {
                        *sentiment += self.risk_market_arguments[k] * indicator;
                    }
                }
                None => {
                }
            }

            // @NOTE: formular to calculate money and stock
            let decison = Investor::tanh(indicator) * Investor::sigmoid(risk);
            let fund_stock = self.fund / self.context.candles[i].c;
            let fund_money = self.fund;

            // @TODO: implement T+3
            if decison > 0.9 && money > fund_money {
                orders.push(self.context.candles[i].c);
                stock += fund_stock;
                money -= fund_money;
            } else if decison < 0.9 && stock > self.fund / self.context.candles[i].c {
                orders.push(-self.context.candles[i].c);
                stock -= fund_stock;
                money += fund_money;
            }
        }

        // @TODO: push investors metadata to redis if needs

        return money + stock*self.context.candles[self.context.candles.len() - 1].c;
    }

    fn gene(&self) -> Vec<f64> {
        let mut ret = Vec::new();
        
        ret.extend(self.market_arguments.clone());
        ret.extend(self.risk_order_arguments.clone());
        ret.extend(self.risk_market_arguments.clone());

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

pub struct SimulatorActor {
    // @NOTE: controller
    controller: Genetic<Investor>,
    cache:     Arc<Addr<RedisActor>>,
    session:   i64,

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
            // @NOTE: controller server
            controller: Genetic::<Investor>::new(
                n_player,
                Self::crossover,
                Self::mutate,
                Self::policy,
            ),

            // @NOTE: cache server
            cache: cache,

            // @NOTE: cache session
            session: 0,

            // @NOTE; template
            lookback_order_history:  lookback_order,
            lookback_candle_history: lookback_history, 
            arg_gen_min:             min, 
            arg_gen_max:             max,
            number_of_player:        n_player,
        }
    }

    fn prepare_sessions(&mut self, number_of_session: usize) -> i64 {
        let first_session = self.session;
        self.session += number_of_session as i64;
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

    fn policy(investor: &Investor) -> bool {
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
        let sampling = Normal::new(0.0, std_dev).unwrap().sample(&mut rng);

        if gene < investor.market_arguments.len() {
            investor.market_arguments[gene] = sampling;
        } else if gene < investor.risk_order_arguments.len() + investor.market_arguments.len() {
            investor.risk_order_arguments[gene - investor.market_arguments.len()] = sampling;
        } else {
            investor.risk_market_arguments[
                gene - investor.market_arguments.len() - investor.risk_order_arguments.len()
            ] = sampling;
        }
    }
}

impl Actor for SimulatorActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct SetupSettingCommand {
    batch_money_for_fund: usize,
    candles: Arc<Vec<CandleStick>>,
    money: f64,
}

impl Handler<SetupSettingCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, msg: SetupSettingCommand, _: &mut Self::Context) -> Self::Result {
        let context = Arc::new(Setting{
            candles:                 msg.candles.clone(),
            money:                   msg.money,
            batch_money_for_fund:    msg.batch_money_for_fund, 
            orders:                  Arc::new(Vec::<f64>::new()),
            lookback_candle_history: self.lookback_candle_history,
            lookback_order_history:  self.lookback_order_history, 
            arg_gen_min:             self.arg_gen_min, 
            arg_gen_max:             self.arg_gen_max,
        });
        let cache = self.cache.clone();
        let n_player = self.number_of_player;

        self.controller.initialize(
            (0..n_player)
                .map(|_| Investor::new(context.clone(), cache.clone()))
                .collect(), 
        );

        Box::pin(async move {
            Ok(None)
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<()>, SimulatorError>")]
pub struct EvaluateFitnessCommand {
    number_of_couple: usize,
    number_of_loop:   usize,
    mutation_rate:    f64,
}

impl Handler<EvaluateFitnessCommand> for SimulatorActor {
    type Result = ResponseFuture<Result<Option<()>, SimulatorError>>;

    fn handle(&mut self, msg: EvaluateFitnessCommand, _: &mut Self::Context) -> Self::Result {
        let first_session = self.prepare_sessions(msg.number_of_loop);

        for i in 0..msg.number_of_loop {
            println!("Loop {}: {}", i, first_session + i as i64);
            self.controller.evolute(msg.number_of_couple, first_session + i as i64);
            self.controller.fluctuate(first_session + i as i64, msg.mutation_rate);

            println!("Avg fitness {}: {}", i, self.controller.average_fitness(first_session + i as i64));
        }

        Box::pin(async move {
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
            200,
            10,
            -100.0,
            100.0,
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
                .map_or(1000000000.0, |money| (*money).parse::<f64>().unwrap());
            let dnse = dnse.clone();

            async move {
                let candles = Arc::new(dnse.send(GetOHCLCommand{
                    resolution: arguments.get("resolution").unwrap().to_string(),
                    stock:      arguments.get("stock").unwrap().to_string(),
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
                })
                .await;
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
                    },
                )
                .await;
            }
        }
    );
}

