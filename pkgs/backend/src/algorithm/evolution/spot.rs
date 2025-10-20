use anyhow::{anyhow, Result};
use nalgebra::DVector;
use rand::Rng;
use std::sync::{Arc, RwLock};

use crate::algorithm::cmaes::{Convex, Sampling};
use crate::algorithm::evolution::{Data, Investor, Phase};
use crate::algorithm::genetic::{Individual, Model};

pub struct Spot {
    data: Arc<RwLock<Data>>,
    phase: Arc<RwLock<Phase>>,
    money: f64,
    stock: f64,
    lifespan: i64,
    stock_holding_period: usize,
    minimum_stock_buy: usize,
    generator: Convex,
}

impl Spot {
    pub fn new(
        data: Arc<RwLock<Data>>,
        money: f64,
        stock: f64,
        lifespan: i64,
        stock_holding_period: usize,
        minimum_stock_buy: usize,
    ) -> Result<Self> {
        data.write()
            .map_err(|error| anyhow!("Failed to read data: {}", error))?
            .shuttle()?;

        // @TODO: cần điều chỉnh lại hằng số này để khống chế số lượng
        //        factors mỗi candles
        let n = 5 * data
            .read()
            .map_err(|error| anyhow!("Failed to read data: {}", error))?
            .window()
            + 1;

        Ok(Self {
            data,
            money,
            stock,
            lifespan,
            stock_holding_period,
            minimum_stock_buy,

            phase: Arc::new(RwLock::new(Phase::Train)),
            generator: Convex::new(n, None, None),
        })
    }
}

impl Model<Investor> for Spot {
    fn random(&self) -> Result<Investor> {
        Ok(Investor::new(
            self.data.clone(),
            self.phase.clone(),
            self.money,
            self.stock,
            self.stock_holding_period,
            self.minimum_stock_buy,
            self.generator.random(),
        ))
    }

    fn mutate(&self, item: &mut Investor, _arguments: &Vec<f64>, index: usize) -> Result<()> {
        let mut rng = rand::thread_rng();
        let mut val = item.get_factor(index) + rng.gen_range(-0.1..0.1);
        val = item.set_factor(index, val);
        item.set_factor(index, val.clamp(-1.0, 1.0));
        Ok(())
    }

    fn crossover(&self, father: &Investor, mother: &Investor) -> Result<Investor> {
        let mut rng = rand::thread_rng();
        let factors_vec: Vec<f64> = father
            .list_factors()
            .iter()
            .zip(mother.list_factors().iter())
            .map(|(f, m)| {
                let base = if rng.gen_bool(0.5) { *f } else { *m };
                (base + rng.gen_range(-0.05..0.05)).clamp(-1.0, 1.0)
            })
            .collect();

        Ok(Investor::new(
            self.data.clone(),
            self.phase.clone(),
            self.money,
            self.stock,
            self.stock_holding_period,
            self.minimum_stock_buy,
            DVector::from(factors_vec),
        ))
    }

    fn extinguish(&self, item: &Individual<Investor>) -> Result<bool> {
        Ok(item.lifetime() > self.lifespan)
    }

    fn validate(&self, population: &Vec<Individual<Investor>>) -> Result<Vec<f64>> {
        let old_phase = *self
            .phase
            .read()
            .map_err(|error| anyhow!("Failed read phase: {}", error))?;

        *self
            .phase
            .write()
            .map_err(|error| anyhow!("Failed write phase: {}", error))? = Phase::Test;

        let fitnesses = population
            .iter()
            .map(|it| it.reevalutate())
            .collect::<Vec<_>>();

        *self
            .phase
            .write()
            .map_err(|error| anyhow!("Failed write phase: {}", error))? = old_phase;
        Ok(fitnesses)
    }

    fn optimize(&mut self, population: &Vec<Individual<Investor>>) -> Result<Vec<f64>> {
        let old_phase = *self
            .phase
            .read()
            .map_err(|error| anyhow!("Failed read phase: {}", error))?;

        *self
            .phase
            .write()
            .map_err(|error| anyhow!("Failed write phase: {}", error))? = Phase::Test;

        let fitnesses = population
            .iter()
            .map(|it| Sampling {
                fitness: it.reevalutate(),
                gene: it.gene(),
            })
            .collect::<Vec<_>>();

        *self
            .phase
            .write()
            .map_err(|error| anyhow!("Failed write phase: {}", error))? = old_phase;

        self.generator.optimize(&fitnesses)?;
        self.data
            .write()
            .map_err(|error| anyhow!("Failed to read data: {}", error))?
            .shuttle()?;

        Ok(fitnesses.iter().map(|it| it.fitness).collect::<Vec<_>>())
    }
}
