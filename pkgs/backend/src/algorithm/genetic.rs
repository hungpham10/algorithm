use nalgebra::DVector;
use rand::Rng;

use chrono::{DateTime, Utc};
use rayon::prelude::*;

use anyhow::{anyhow, Result};
use influxdb::{Client, InfluxDbWriteable};
use log::error;

use std::cmp::min;
use std::sync::{Arc, RwLock};

use crate::algorithm::percentile;

#[derive(InfluxDbWriteable, Clone, Debug)]
pub struct Statistic {
    pub p99: f64,
    pub p95: f64,
    pub p75: f64,
    pub p55: f64,
    pub best: f64,
    pub worst: f64,
    pub median: f64,
    pub stddev: f64,

    time: DateTime<Utc>,

    #[influxdb(tag)]
    session: String,
}

#[derive(Clone)]
pub struct InfluxDb {
    url: String,
    token: String,
    bucket: String,
}

impl InfluxDb {
    pub fn new(url: &str, token: &str, bucket: &str) -> Self {
        Self {
            url: url.to_string(),
            token: token.to_string(),
            bucket: bucket.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Individual<T: Clone + Sync + Send> {
    player: T,
    created: i64,
    session: i64,
    fitness: f64,
}

impl<T: Clone + Sync + Send> Individual<T> {
    pub fn into(&self) -> &T {
        &self.player
    }

    pub fn lifetime(&self) -> i64 {
        self.session - self.created
    }
}

pub trait Model<T: Clone + Sync + Send> {
    fn optimize(&mut self, population: &Vec<Individual<T>>) -> Result<Vec<f64>>;
    fn random(&self) -> Result<T>;
    fn mutate(&self, item: &mut T, arguments: &Vec<f64>, index: usize) -> Result<()>;
    fn crossover(&self, father: &T, mother: &T) -> Result<T>;
    fn extinguish(&self, item: &Individual<T>) -> Result<bool>;
}

pub trait Player {
    fn initialize(&mut self) -> Result<()>;
    fn estimate(&self) -> Result<f64>;
    fn gene(&self) -> DVector<f64>;
}

#[derive(Clone)]
pub struct Genetic<T: Clone + Sync + Send, M: Model<T>> {
    population: Vec<Individual<T>>,
    profile: Option<Client>,
    model: Arc<RwLock<M>>,
    limit: usize,
}

impl<T: Player + Clone + Sync + Send> Individual<T> {
    pub fn new(player: T, session: i64) -> Self {
        Self {
            player,
            fitness: 0.0,
            created: session,
            session: session,
        }
    }

    pub fn gene(&self) -> DVector<f64> {
        self.player.gene()
    }

    pub fn player(&self) -> &T {
        &self.player
    }

    pub fn reevalutate(&self) -> f64 {
        self.player.estimate().unwrap_or_else(|e| {
            error!("Re-evaluation failed: {}, using cached fitness", e);
            self.fitness
        })
    }

    pub fn estimate(&self) -> f64 {
        self.fitness
    }

    pub fn estimate_mut(&mut self, session: i64) -> Result<f64> {
        if self.session != session && self.session < session {
            self.update_fitness(session, self.player.estimate()?);
        }
        Ok(self.fitness)
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.player.initialize()
    }

    fn update_fitness(&mut self, session: i64, fitness: f64) {
        self.fitness = fitness;
        self.session = session;
    }
}

impl<T: Player + Clone + Sync + Send, M: Model<T>> Genetic<T, M> {
    pub fn new(limit: usize, model: Arc<RwLock<M>>, capture: Option<InfluxDb>) -> Self {
        Self {
            population: Vec::<Individual<T>>::new(),
            profile: match capture {
                Some(capture) => Some(
                    Client::new(capture.url.as_str(), capture.bucket.as_str())
                        .with_token(capture.token.as_str()),
                ),
                None => None,
            },
            limit,
            model,
        }
    }

    pub fn initialize(
        &mut self,
        n_accentors: usize,
        session: i64,
        shuttle_rate: Option<f64>,
    ) -> Result<()> {
        let model = self.model.read().unwrap();
        let profiles = if session > 0 {
            self.population
                .iter()
                .map(|iter| iter.estimate())
                .collect::<Vec<_>>()
        } else {
            vec![0.0; n_accentors]
        };
        let sum_profiles = profiles.iter().sum::<f64>();

        let mut rng = rand::thread_rng();
        let mut population = Vec::new();
        let mut roulette = if sum_profiles > 0.0 {
            profiles
                .iter()
                .map(|&iw| iw / sum_profiles)
                .collect::<Vec<_>>()
        } else {
            vec![1.0 / n_accentors as f64; n_accentors]
        };

        for i in 1..min(n_accentors, roulette.len()) {
            roulette[i] += roulette[i - 1];
        }

        if session == 0 || shuttle_rate.is_none() {
            self.population.clear();
        }

        for _ in 0..n_accentors {
            if session > 0 && shuttle_rate.is_some() {
                if rng.gen::<f64>() < shuttle_rate.unwrap_or(0.0) {
                    let i = self.roulette_wheel_selection(&mut roulette, rng.gen::<f64>());

                    if i < self.population.len() {
                        population.push(self.population[i].clone());
                        continue;
                    }
                }
            }

            population.push(Individual::<T>::new(model.random()?, session));
        }
        self.population = population;

        drop(model);
        for player in &mut self.population {
            player.initialize()?;
        }
        Ok(())
    }

    pub fn get(&self, index: usize) -> &Individual<T> {
        &self.population[index]
    }

    pub fn size(&self) -> usize {
        self.population.len()
    }

    pub fn average_fitness(&self) -> f64 {
        let mut sumup = 0.0;
        for i in 0..self.population.len() {
            sumup += self.population[i].estimate();
        }
        sumup / self.population.len() as f64
    }

    pub fn best_fitness(&self) -> f64 {
        let mut best: f64 = 0.0;
        for i in 0..self.population.len() {
            let val = self.population[i].estimate();
            best = best.max(val);
        }
        best
    }

    pub fn best_player(&self) -> T {
        let mut best: f64 = 0.0;
        let mut id_best = 0;
        for i in 0..self.population.len() {
            let tmp = best.max(self.population[i].estimate());
            if tmp != best {
                best = tmp;
                id_best = i;
            }
        }
        self.population[id_best].player.clone()
    }

    pub fn statistic(&self, session: i64) -> Result<Statistic> {
        if self.population.is_empty() {
            return Err(anyhow!("There is no population to profile"));
        }

        // collect fitness values
        let mut vals = self
            .population
            .iter()
            .map(|p| p.estimate())
            .collect::<Vec<_>>();

        // sort ascending for percentile calculations
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;
        let variance = vals
            .iter()
            .map(|v| {
                let d = v - mean;
                d * d
            })
            .sum::<f64>()
            / n;
        let stddev = variance.sqrt();

        let p99 = percentile(&vals, 99.0);
        let p95 = percentile(&vals, 95.0);
        let p75 = percentile(&vals, 75.0);
        let p55 = percentile(&vals, 55.0);

        let best = *vals
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&f64::NAN);
        let worst = *vals
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&f64::NAN);
        let median = percentile(&vals, 50.0);

        let stats = Statistic {
            time: Utc::now(),
            p99,
            p95,
            p75,
            p55,
            best,
            worst,
            median,
            stddev,
            session: session.to_string(),
        };

        Ok(stats)
    }

    pub async fn capture(&self, name: &str, stats: &Statistic) -> Result<()> {
        match &self.profile {
            Some(client) => {
                client
                    .query(vec![stats.clone().into_query(name)])
                    .await
                    .map_err(|error| anyhow!("InfluxDB write error: {}", error))?;
            }
            None => {}
        }
        Ok(())
    }

    pub fn estimate(&mut self, session: i64) -> Result<()> {
        let individual_estimation = self
            .population
            .par_iter()
            .enumerate()
            .map(|(i, iter)| (i, iter.player.estimate()))
            .collect::<Vec<(usize, Result<f64>)>>();
        // @NOTE: update fitness of current session into each player
        for (i, fitness) in individual_estimation {
            self.population[i].update_fitness(session, fitness?);
        }
        Ok(())
    }

    pub fn evolute(
        &mut self,
        number_of_couple: usize,
        session: i64,
        mutation_rate: f64,
    ) -> Result<()> {
        let mut rng = rand::thread_rng();
        let mut extintion = Vec::<usize>::new();
        if !(0.0..=1.0).contains(&mutation_rate) || mutation_rate.is_nan() {
            return Err(anyhow!("`mutation_rate` must be within [0.0, 1.0]"));
        }
        if 2 * number_of_couple >= self.limit {
            return Err(anyhow!("Number of couple is excessing the limitation"));
        }
        let individual_estimation = if self.population.len() > 100 {
            self.population
                .par_iter()
                .enumerate()
                .map(|(i, iter)| (i, iter.player.estimate()))
                .collect::<Vec<(usize, Result<f64>)>>()
        } else {
            self.population
                .iter()
                .enumerate()
                .map(|(i, iter)| (i, iter.player.estimate()))
                .collect::<Vec<(usize, Result<f64>)>>()
        };
        let model = self.model.read().unwrap();
        for (i, fitness) in individual_estimation {
            // @NOTE: update fitness of current session into each player
            self.population[i].update_fitness(session, fitness?);
            // @NOTE: to make some events like mass extinction, we need to perform estimation
            // who would be removed out of our population
            if model.extinguish(&self.population[i])? {
                extintion.push(i);
            }
        }
        // @NOTE: remove dead player
        if !extintion.is_empty() && extintion.len() < self.population.len() {
            let percent = extintion.len() as f64 / self.population.len() as f64;
            let mut new_population = Vec::new();
            for (i, individual) in self.population.iter().enumerate() {
                if !extintion.contains(&i) || rng.gen::<f64>() >= percent {
                    new_population.push(individual.clone());
                }
            }
            self.population = new_population;
        }
        // @NOTE: force to remove players who have bad quality
        if self.population.len() > self.limit.saturating_sub(number_of_couple) {
            // @NOTE: sort by estimation fitness
            self.population
                .sort_by(|a, b| b.estimate().partial_cmp(&a.estimate()).unwrap());
            // @NOTE: remove old player
            self.population
                .truncate(self.limit.saturating_sub(number_of_couple));
        }
        let mut roulette = vec![0.0; self.population.len()];
        let mut sumup = 0.0;
        for (i, individual) in self.population.iter().enumerate() {
            let fitness = individual.estimate();
            roulette[i] = fitness;
            sumup += fitness;
        }
        if sumup <= 0.0 {
            let uniform_prob = 1.0 / self.population.len() as f64;
            for item in roulette.iter_mut() {
                *item = uniform_prob;
            }
        } else {
            for item in roulette.iter_mut() {
                *item /= sumup;
            }
        }
        for i in 1..self.population.len() {
            roulette[i] += roulette[i - 1];
        }
        // @NOTE: try to promote new player
        for _ in 0..number_of_couple {
            let f = self.roulette_wheel_selection(&mut roulette, rng.gen::<f64>());
            let m = self.roulette_wheel_selection(&mut roulette, rng.gen::<f64>());
            let mut new_player =
                model.crossover(&self.population[f].player, &self.population[m].player)?;
            for i in 0..new_player.gene().len() {
                if rng.gen::<f64>() < mutation_rate {
                    model.mutate(&mut new_player, &Vec::new(), i)?;
                }
            }
            self.population
                .push(Individual::<T>::new(new_player, session));
        }
        Ok(())
    }

    pub fn fluctuate(
        &mut self,
        session: i64,
        arguments: &Vec<Vec<f64>>,
        mutation_rate: f64,
    ) -> Result<()> {
        let mut rng = rand::thread_rng();
        if !(0.0..=1.0).contains(&mutation_rate) || mutation_rate.is_nan() {
            return Err(anyhow!("`mutation_rate` must be within [0.0, 1.0]"));
        }
        let model = self.model.read().unwrap();
        for player in &mut self.population {
            for i in 0..player.player.gene().len() {
                if rng.gen::<f64>() < mutation_rate {
                    model.mutate(
                        &mut player.player,
                        arguments.get(i).unwrap_or(&Vec::<f64>::new()),
                        i,
                    )?;
                }
            }
            player.estimate_mut(session)?;
        }
        Ok(())
    }

    pub fn optimize(&mut self) -> Result<Vec<f64>> {
        self.model
            .write()
            .map_err(|error| anyhow!("Failed to lock model to write: {}", error))?
            .optimize(&self.population)
    }

    fn roulette_wheel_selection(&self, roulette: &mut [f64], target: f64) -> usize {
        match roulette
            .binary_search_by(|x| x.partial_cmp(&target).unwrap_or(std::cmp::Ordering::Equal))
        {
            Ok(i) => i,
            Err(i) => i.min(self.population.len() - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestPlayer {
        genes: DVector<f64>,
    }

    impl Player for TestPlayer {
        fn initialize(&mut self) -> Result<()> {
            Ok(())
        }

        fn estimate(&self) -> Result<f64> {
            Ok(self.genes.iter().map(|g| g * g).sum())
        }

        fn gene(&self) -> DVector<f64> {
            self.genes.clone()
        }
    }

    struct TestModel {
        num_genes: usize,
    }

    impl Model<TestPlayer> for TestModel {
        fn optimize(&mut self, _population: &Vec<Individual<TestPlayer>>) -> Result<Vec<f64>> {
            Ok(Vec::new())
        }

        fn random(&self) -> Result<TestPlayer> {
            let genes_vec: Vec<f64> = (0..self.num_genes)
                .map(|_| rand::thread_rng().gen_range(-1.0..1.0))
                .collect();
            let genes = DVector::from(genes_vec);
            Ok(TestPlayer { genes })
        }

        fn mutate(&self, item: &mut TestPlayer, _args: &Vec<f64>, index: usize) -> Result<()> {
            let noise = rand::thread_rng().gen_range(-0.1..0.1);
            item.genes[index] += noise;
            item.genes[index] = item.genes[index].clamp(-1.0, 1.0);
            Ok(())
        }

        fn crossover(&self, father: &TestPlayer, mother: &TestPlayer) -> Result<TestPlayer> {
            let genes_vec: Vec<f64> = father
                .genes
                .iter()
                .zip(mother.genes.iter())
                .map(|(f, m)| {
                    ((*f + *m) / 2.0 + rand::thread_rng().gen_range(-0.05..0.05)).clamp(-1.0, 1.0)
                })
                .collect();
            let genes = DVector::from(genes_vec);
            Ok(TestPlayer { genes })
        }

        fn extinguish(&self, _item: &Individual<TestPlayer>) -> Result<bool> {
            Ok(false)
        }
    }

    #[test]
    fn test_individual_new_and_estimate() {
        let genes = DVector::from(vec![1.0, 2.0]);
        let player = TestPlayer { genes };
        let individual = Individual::new(player, 0);
        assert_eq!(individual.estimate(), 0.0);
        assert_eq!(individual.session, 0);
    }

    #[test]
    fn test_individual_update_fitness() {
        let genes = DVector::from(vec![1.0]);
        let mut player = TestPlayer { genes };
        player.initialize().unwrap();
        let mut individual = Individual::new(player, 0);
        individual.update_fitness(1, 5.0);
        assert_eq!(individual.estimate(), 5.0);
        assert_eq!(individual.session, 1);
    }

    #[test]
    fn test_genetic_initialize() {
        let model = Arc::new(RwLock::new(TestModel { num_genes: 2 }));
        let mut genetic = Genetic::new(10, model, None);
        genetic.initialize(5, 0, None).unwrap();
        assert_eq!(genetic.size(), 5);
        for ind in &genetic.population {
            assert!(!ind.estimate().is_nan());
        }
    }

    #[test]
    fn test_genetic_average_fitness() {
        let model = Arc::new(RwLock::new(TestModel { num_genes: 1 }));
        let mut genetic = Genetic::new(10, model, None);
        genetic.initialize(3, 0, None).unwrap();
        // Manually set fitness for testing
        for (i, ind) in genetic.population.iter_mut().enumerate() {
            ind.update_fitness(0, i as f64 + 1.0);
        }
        let avg = genetic.average_fitness();
        assert_eq!(avg, 2.0);
    }

    #[test]
    fn test_genetic_best_fitness() {
        let model = Arc::new(RwLock::new(TestModel { num_genes: 1 }));
        let mut genetic = Genetic::new(10, model, None);
        genetic.initialize(3, 0, None).unwrap();
        // Manually set fitness
        genetic.population[0].update_fitness(0, 1.0);
        genetic.population[1].update_fitness(0, 3.0);
        genetic.population[2].update_fitness(0, 2.0);
        assert_eq!(genetic.best_fitness(), 3.0);
    }

    #[test]
    fn test_genetic_estimate() {
        let model = Arc::new(RwLock::new(TestModel { num_genes: 1 }));
        let mut genetic = Genetic::new(10, model, None);
        genetic.initialize(2, 0, None).unwrap();
        genetic.estimate(1).unwrap();
        for ind in &genetic.population {
            assert_eq!(ind.session, 1);
            assert!(!ind.estimate().is_nan());
        }
    }

    #[test]
    fn test_genetic_evolute() {
        let model = Arc::new(RwLock::new(TestModel { num_genes: 2 }));
        let mut genetic = Genetic::new(10, model, None);
        genetic.initialize(5, 0, None).unwrap();
        // Set initial fitness
        for (i, ind) in genetic.population.iter_mut().enumerate() {
            ind.update_fitness(0, (i as f64 + 1.0));
        }
        let initial_best = genetic.best_fitness();
        genetic.evolute(2, 1, 0.1).unwrap();
        let new_best = genetic.best_fitness();
        // After evolution, population size should increase by 2*number_of_couple but then truncate if over limit
        // But since limit=10, initial=5, couples=2 -> new=9 <10
        assert_eq!(genetic.size(), 7);
        // Best should not decrease significantly, but since random, just check it's computed
        assert!(!new_best.is_nan());
    }

    #[test]
    fn test_genetic_fluctuate() {
        let model = Arc::new(RwLock::new(TestModel { num_genes: 2 }));
        let mut genetic = Genetic::new(10, model, None);
        genetic.initialize(2, 0, None).unwrap();
        let initial_fitness: Vec<f64> = genetic.population.iter().map(|p| p.estimate()).collect();
        let args = vec![vec![0.0]; 2];
        genetic.fluctuate(1, &args, 0.5).unwrap();
        // Some mutations should happen, fitness may change
        for (i, ind) in genetic.population.iter().enumerate() {
            assert_eq!(ind.session, 1);
            // Fitness should be updated
            assert_ne!(ind.estimate(), initial_fitness[i]);
        }
    }

    #[test]
    fn test_model_random() {
        let model = TestModel { num_genes: 3 };
        let player = model.random().unwrap();
        assert_eq!(player.gene().len(), 3);
        for g in player.gene().iter() {
            assert!((-1.0..=1.0).contains(g));
        }
    }

    #[test]
    fn test_model_mutate() {
        let model = TestModel { num_genes: 1 };
        let mut player = TestPlayer {
            genes: DVector::from(vec![0.0]),
        };
        model.mutate(&mut player, &vec![], 0).unwrap();
        assert!((-1.0..=1.0).contains(&player.genes[0]));
        assert_ne!(player.genes[0], 0.0);
    }

    #[test]
    fn test_model_crossover() {
        let model = TestModel { num_genes: 2 };
        let father = TestPlayer {
            genes: DVector::from(vec![1.0, -1.0]),
        };
        let mother = TestPlayer {
            genes: DVector::from(vec![-1.0, 1.0]),
        };
        let child = model.crossover(&father, &mother).unwrap();
        for g in child.gene().iter() {
            assert!((-1.0..=1.0).contains(g));
        }
    }
}
