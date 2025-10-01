use anyhow::{anyhow, Result};
use rand::Rng;
use rayon::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Individual<T: Clone + Sync + Send> {
    player: T,
    session: i64,
    fitness: f64,
}

impl<T: Clone + Sync + Send> Individual<T> {
    pub fn into(&self) -> &T {
        &self.player
    }
}

pub trait Model<T: Clone + Sync + Send> {
    fn random(&self) -> Result<T>;
    fn mutate(&self, item: &mut T, arguments: &Vec<f64>, index: usize) -> Result<()>;
    fn crossover(&self, father: &T, mother: &T) -> Result<T>;
    fn extinguish(&self, item: &Individual<T>) -> Result<bool>;
}

pub trait Player {
    fn initialize(&mut self) -> Result<()>;
    fn estimate(&self) -> f64;
    fn gene(&self) -> Vec<f64>;
}

#[derive(Clone)]
pub struct Genetic<T: Clone + Sync + Send, M: Model<T>> {
    population: Vec<Individual<T>>,
    model: Arc<M>,
    limit: usize,
}

impl<T: Player + Clone + Sync + Send> Individual<T> {
    pub fn new(player: T) -> Self {
        Self {
            player,
            fitness: 0.0,
            session: -1,
        }
    }

    pub fn player(&self) -> &T {
        &self.player
    }

    pub fn estimate(&self) -> f64 {
        self.fitness
    }

    pub fn estimate_mut(&mut self, session: i64) -> f64 {
        if self.session != session && self.session < session {
            self.update_fitness(session, self.player.estimate());
        }

        self.fitness
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
    pub fn new(limit: usize, model: Arc<M>) -> Self {
        Self {
            population: Vec::<Individual<T>>::new(),
            limit,
            model,
        }
    }

    pub fn initialize(&mut self, n_accentors: usize) -> Result<()> {
        self.population.clear();

        for _ in 0..n_accentors {
            self.population
                .push(Individual::<T>::new(self.model.random()?));
        }

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

    pub fn average_fitness(&self, session: i64) -> f64 {
        let mut sumup = 0.0;

        for i in 0..self.population.len() {
            sumup += self.population[i].estimate();
        }

        sumup / self.population.len() as f64
    }

    pub fn best_fitness(&self, session: i64) -> f64 {
        let mut best: f64 = 0.0;

        for i in 0..self.population.len() {
            let val = self.population[i].estimate();
            best = best.max(val);
        }

        best
    }

    pub fn best_player(&self, session: i64) -> T {
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

    pub fn estimate(&mut self, session: i64) {
        let individual_estimation = self
            .population
            .par_iter()
            .enumerate()
            .map(|(i, iter)| (i, iter.player.estimate()))
            .collect::<Vec<(usize, f64)>>();

        for (i, fitness) in individual_estimation {
            // @NOTE: update fitness of current session into each player
            self.population[i].update_fitness(session, fitness);
        }
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
                .collect::<Vec<(usize, f64)>>()
        } else {
            self.population
                .iter()
                .enumerate()
                .map(|(i, iter)| (i, iter.player.estimate()))
                .collect::<Vec<(usize, f64)>>()
        };

        for (i, fitness) in individual_estimation {
            // @NOTE: update fitness of current session into each player
            self.population[i].update_fitness(session, fitness);

            // @NOTE: to make some events like mass extinction, we need to perform estimation
            //        who would be removed out of our population
            if self.model.extinguish(&self.population[i])? {
                extintion.push(i);
            }
        }

        // @NOTE: remove dead player
        if !extintion.is_empty() && extintion.len() < self.population.len() {
            let percent = extintion.len() as f64 / self.population.len() as f64;
            let mut new_population = Vec::with_capacity(self.population.len());
            for (i, individual) in self.population.drain(..).enumerate() {
                if !extintion.contains(&i) || rng.gen::<f64>() >= percent {
                    new_population.push(individual);
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
            let mut new_player = self
                .model
                .crossover(&self.population[f].player, &self.population[m].player)?;
            for i in 0..new_player.gene().len() {
                if rng.gen::<f64>() < mutation_rate {
                    self.model.mutate(&mut new_player, &Vec::new(), i)?;
                }
            }
            self.population.push(Individual::<T>::new(new_player));
        }

        Ok(())
    }

    pub fn fluctuate(
        &mut self,
        session: i64,
        arguments: Vec<Vec<f64>>,
        mutation_rate: f64,
    ) -> Result<()> {
        let mut rng = rand::thread_rng();

        if !(0.0..=1.0).contains(&mutation_rate) || mutation_rate.is_nan() {
            return Err(anyhow!("`mutation_rate` must be within [0.0, 1.0]"));
        }

        for player in &mut self.population {
            for i in 0..player.player.gene().len() {
                if rng.gen::<f64>() < mutation_rate {
                    self.model.mutate(
                        &mut player.player,
                        arguments.get(i).unwrap_or(&Vec::<f64>::new()),
                        i,
                    )?;
                }
            }

            player.estimate_mut(session);
        }
        Ok(())
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
    use rand::rngs::ThreadRng;
    use rand::Rng;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestPlayer {
        genes: [f64; 3],
        fitness: f64,
    }

    impl TestPlayer {
        fn new(fitness: f64) -> Self {
            TestPlayer {
                genes: [0.0, 0.0, 0.0],
                fitness,
            }
        }
    }

    impl Player for TestPlayer {
        fn initialize(&mut self) -> Result<()> {
            self.genes = [1.0, 2.0, 3.0];
            Ok(())
        }

        fn estimate(&self) -> f64 {
            self.fitness
        }

        fn gene(&self) -> Vec<f64> {
            self.genes.to_vec()
        }
    }

    unsafe impl Sync for TestPlayer {}
    unsafe impl Send for TestPlayer {}

    struct TestModel {
        rng: ThreadRng,
    }

    impl TestModel {
        fn new() -> Self {
            let mut rng = rand::thread_rng();

            Self { rng }
        }
    }
    impl Model<TestPlayer> for TestModel {
        fn random(&self) -> Result<TestPlayer> {
            Ok(TestPlayer::new(self.gen::<f64>() * 10.0 - 5.0))
        }

        fn crossover(&self, p1: &TestPlayer, _p2: &TestPlayer) -> Result<TestPlayer> {
            Ok(p1.clone())
        }

        fn mutate(&self, _player: &mut TestPlayer, _args: &Vec<f64>, _idx: usize) -> Result<()> {
            Ok(())
        }

        fn extinguish(&self, _individual: &Individual<TestPlayer>) -> Result<bool> {
            Ok(false)
        }
    }

    #[test]
    fn test_evolute_normal_operation() {
        let mut genetic = Genetic::new(5, Arc::new(TestModel {}));
        genetic.initialize(4).unwrap();

        let session = 1;
        let number_of_couple = 2;
        let mutation_rate = 0.1;

        genetic
            .evolute(number_of_couple, session, mutation_rate)
            .unwrap();

        assert_eq!(genetic.size(), 5);
        let avg_fitness = genetic.average_fitness(session);
        assert!(avg_fitness > 0.0, "Average fitness should be positive");
        let best_fitness = genetic.best_fitness(session);
        assert!(
            best_fitness <= 4.0,
            "Best fitness should not exceed initial max"
        );
    }

    #[test]
    fn test_evolute_truncation() {
        let mut genetic = Genetic::new(3, Arc::new(TestModel {}));
        genetic.initialize(4).unwrap();

        let session = 1;
        let number_of_couple = 1;
        let mutation_rate = 0.1;

        genetic
            .evolute(number_of_couple, session, mutation_rate)
            .unwrap();

        assert_eq!(genetic.size(), 3, "Population should be truncated to limit");
        let best_fitness = genetic.best_fitness(session);
        assert!(
            best_fitness >= 3.0,
            "Best fitness should be from top individuals"
        );
    }

    #[test]
    fn test_roulette_wheel_selection() {
        let mut genetic = Genetic::new(5, Arc::new(TestModel {}));
        genetic.initialize(3).unwrap();

        let mut roulette = vec![1.0, 3.0, 6.0];
        let sumup = 6.0;
        for item in roulette.iter_mut() {
            *item /= sumup;
        }
        for i in 1..roulette.len() {
            roulette[i] += roulette[i - 1];
        }

        let idx = genetic.roulette_wheel_selection(&mut roulette[..], 0.1);
        assert_eq!(idx, 0, "Should select first individual for low target");

        let idx = genetic.roulette_wheel_selection(&mut roulette[..], 0.5);
        assert_eq!(idx, 1, "Should select second individual for mid target");

        let idx = genetic.roulette_wheel_selection(&mut roulette[..], 0.9);
        assert_eq!(idx, 2, "Should select third individual for high target");
    }

    #[test]
    fn test_fluctuate_mutation() {
        let mut genetic = Genetic::new(5, Arc::new(TestModel {}));
        genetic.initialize(2).unwrap();

        let session = 1;
        let mutation_rate = 1.0;
        let arguments = vec![vec![0.0], vec![0.0], vec![0.0]];

        let _ = genetic.fluctuate(session, arguments, mutation_rate);

        let avg_fitness = genetic.average_fitness(session);
        assert!(
            avg_fitness > 0.0,
            "Fitness should be updated after fluctuation"
        );
    }

    #[test]
    fn test_best_player() {
        let mut genetic = Genetic::new(5, Arc::new(TestModel {}));
        genetic.initialize(3).unwrap();

        let session = 1;
        genetic.estimate(session);

        let best = genetic.best_player(session);
        assert_eq!(best.fitness, 4.0, "Best player should have highest fitness");
    }
}
