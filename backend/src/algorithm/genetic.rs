use anyhow::{anyhow, Result};
use rand::Rng;
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct Individual<T: Clone + Sync + Send> {
    player: T,
    session: i64,
    fitness: f64,
}

pub type CrossoverCallback<T> = fn(&Genetic<T>, &T, usize, &T, usize, i64) -> T;
pub type MutateCallback<T> = fn(&mut T, &Vec<f64>, usize);
pub type ExtintionCallback<T> = fn(&Individual<T>, i64) -> bool;

impl<T: Clone + Sync + Send> Individual<T> {
    pub fn into(&self) -> &T {
        &self.player
    }
}

pub trait Player {
    fn initialize(&mut self);
    fn estimate(&self) -> f64;
    fn gene(&self) -> Vec<f64>;
}

#[derive(Clone, Debug)]
pub struct Genetic<T: Clone + Sync + Send> {
    population: Vec<Individual<T>>,
    limit: usize,
    crossover: CrossoverCallback<T>,
    extinguish: ExtintionCallback<T>,
    mutate: MutateCallback<T>,
}

impl<T: Player + Clone + Sync + Send + std::fmt::Debug> Individual<T> {
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

    pub fn estimate(&self, session: i64) -> f64 {
        if session != self.session {
            return 0.0;
        }

        self.fitness
    }

    pub fn estimate_mut(&mut self, session: i64) -> f64 {
        if self.session != session && self.session < session {
            self.update_fitness(session, self.player.estimate());
        }

        self.fitness
    }

    pub fn initialize(&mut self) {
        self.player.initialize();
    }

    fn update_fitness(&mut self, session: i64, fitness: f64) {
        self.fitness = fitness;
        self.session = session;
    }
}

impl<T: Player + Clone + Sync + Send + std::fmt::Debug> Genetic<T> {
    pub fn new(
        limit: usize,
        crossover: CrossoverCallback<T>,
        mutate: MutateCallback<T>,
        extinguish: ExtintionCallback<T>,
    ) -> Self {
        Self {
            population: Vec::<Individual<T>>::new(),
            limit,
            crossover,
            mutate,
            extinguish,
        }
    }

    pub fn initialize(&mut self, players: Vec<T>) {
        self.population.clear();

        for player in players {
            self.population.push(Individual::<T>::new(player.clone()));
        }

        for player in &mut self.population {
            player.initialize();
        }
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
            sumup += self.population[i].estimate(session);
        }

        sumup / self.population.len() as f64
    }

    pub fn best_fitness(&self, session: i64) -> f64 {
        let mut best: f64 = 0.0;

        for i in 0..self.population.len() {
            let val = self.population[i].estimate(session);
            best = best.max(val);
        }

        best
    }

    pub fn best_player(&self, session: i64) -> T {
        let mut best: f64 = 0.0;
        let mut id_best = 0;

        for i in 0..self.population.len() {
            let tmp = best.max(self.population[i].estimate(session));
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
            if (self.extinguish)(&self.population[i], session) {
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
            self.population.sort_by(|a, b| {
                b.estimate(session)
                    .partial_cmp(&a.estimate(session))
                    .unwrap()
            });

            // @NOTE: remove old player
            self.population
                .truncate(self.limit.saturating_sub(number_of_couple));
        }

        let mut roulette = vec![0.0; self.population.len()];
        let mut sumup = 0.0;

        for (i, individual) in self.population.iter().enumerate() {
            let fitness = individual.estimate(session);
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
            let mut new_player = (self.crossover)(
                self,
                &self.population[f].player,
                f,
                &self.population[m].player,
                m,
                session,
            );
            for i in 0..new_player.gene().len() {
                if rng.gen::<f64>() < mutation_rate {
                    (self.mutate)(&mut new_player, &Vec::new(), i);
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
                    (self.mutate)(
                        &mut player.player,
                        arguments.get(i).unwrap_or(&Vec::<f64>::new()),
                        i,
                    );
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
        fn initialize(&mut self) {
            self.genes = [1.0, 2.0, 3.0];
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

    fn test_crossover<T: Player + Clone + Sync + Send>(
        _genetic: &Genetic<T>,
        p1: &T,
        _p1_idx: usize,
        _p2: &T,
        _p2_idx: usize,
        _session: i64,
    ) -> T {
        p1.clone()
    }

    fn test_mutate<T: Clone + Sync + Send>(_player: &mut T, _args: &Vec<f64>, _idx: usize) {}

    fn test_extinguish<T: Clone + Sync + Send>(_individual: &Individual<T>, _session: i64) -> bool {
        false
    }

    fn test_extinguish_all<T: Clone + Sync + Send>(
        _individual: &Individual<T>,
        _session: i64,
    ) -> bool {
        true
    }

    #[test]
    fn test_evolute_normal_operation() {
        let mut genetic = Genetic::new(
            5,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish::<TestPlayer>,
        );
        let players = vec![
            TestPlayer::new(1.0),
            TestPlayer::new(2.0),
            TestPlayer::new(3.0),
            TestPlayer::new(4.0),
        ];
        genetic.initialize(players);

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
    fn test_evolute_zero_fitness() {
        let mut genetic = Genetic::new(
            5,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish::<TestPlayer>,
        );
        let players = vec![
            TestPlayer::new(0.0),
            TestPlayer::new(0.0),
            TestPlayer::new(0.0),
        ];
        genetic.initialize(players);

        let session = 1;
        let number_of_couple = 2;
        let mutation_rate = 0.1;

        genetic
            .evolute(number_of_couple, session, mutation_rate)
            .unwrap();

        assert_eq!(
            genetic.size(),
            5,
            "Population should grow by number_of_couple"
        );
        let avg_fitness = genetic.average_fitness(session);
        assert_eq!(avg_fitness, 0.0);
    }

    #[test]
    fn test_evolute_extinction() {
        let mut genetic = Genetic::new(
            5,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish_all::<TestPlayer>,
        );
        let players = vec![
            TestPlayer::new(1.0),
            TestPlayer::new(2.0),
            TestPlayer::new(3.0),
        ];
        genetic.initialize(players);

        let session = 1;
        let number_of_couple = 1;
        let mutation_rate = 0.1;

        genetic
            .evolute(number_of_couple, session, mutation_rate)
            .unwrap();

        assert!(genetic.size() > 0, "Population should not be empty");
        assert!(
            genetic.size() <= 3 + number_of_couple,
            "Population should not exceed initial + new"
        );
    }

    #[test]
    fn test_evolute_truncation() {
        let mut genetic = Genetic::new(
            3,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish::<TestPlayer>,
        );
        let players = vec![
            TestPlayer::new(1.0),
            TestPlayer::new(2.0),
            TestPlayer::new(3.0),
            TestPlayer::new(4.0),
        ];
        genetic.initialize(players);

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
        let mut genetic = Genetic::new(
            5,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish::<TestPlayer>,
        );
        let players = vec![
            TestPlayer::new(1.0),
            TestPlayer::new(2.0),
            TestPlayer::new(3.0),
        ];
        genetic.initialize(players);

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
        let mut genetic = Genetic::new(
            5,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish::<TestPlayer>,
        );
        let players = vec![TestPlayer::new(1.0), TestPlayer::new(2.0)];
        genetic.initialize(players);

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
        let mut genetic = Genetic::new(
            5,
            test_crossover::<TestPlayer>,
            test_mutate::<TestPlayer>,
            test_extinguish::<TestPlayer>,
        );
        let players = vec![
            TestPlayer::new(1.0),
            TestPlayer::new(4.0),
            TestPlayer::new(2.0),
        ];
        genetic.initialize(players);

        let session = 1;
        genetic.estimate(session);

        let best = genetic.best_player(session);
        assert_eq!(best.fitness, 4.0, "Best player should have highest fitness");
    }
}
