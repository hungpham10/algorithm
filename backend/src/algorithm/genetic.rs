use rayon::prelude::*;
use rand::Rng;

#[derive(Clone, Debug)]
pub struct Individual<T: Clone + Sync + Send> {
    player:  T,
    session: i64,
    fitness: f64,
}

pub type CrossoverCallback<T> = fn(&Genetic<T>, &T, usize, &T, usize, i64) -> T;
pub type MutateCallback<T> = fn(&mut T, &Vec<f64>, usize);
pub type ExtintionCallback<T> = fn(&Individual<T>, i64) -> bool;

impl <T: Clone + Sync + Send> Individual<T> {
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
    limit:      usize,
    crossover:  CrossoverCallback<T>,
    extinguish: ExtintionCallback<T>,
    mutate:     MutateCallback<T>,
}

impl <T: Player + Clone + Sync + Send + std::fmt::Debug> Individual<T> {
    pub fn new(player: T) -> Self {
        Self{
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

        return self.fitness;
    }

    pub fn estimate_mut(&mut self, session: i64) -> f64 {
        if self.session != session && self.session < session {
            self.update_fitness(session, self.player.estimate());
        }

        return self.fitness
    }

    pub fn initialize(&mut self) {
        self.player.initialize();
    }

    fn update_fitness(&mut self, session: i64, fitness: f64) {
        self.fitness = fitness;
        self.session = session;
    }
}

impl <T: Player + Clone + Sync + Send + std::fmt::Debug> Genetic<T> {
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
            self.population.push(
                Individual::<T>::new(
                    player.clone()
                )
            );
        }

        for player in &mut self.population {
            player.initialize();
        }
    }

    pub fn get(&self, index: usize) -> &Individual<T> {
        return &self.population[index];
    }

    pub fn size(&self) -> usize {
        self.population.len()
    }

    pub fn average_fitness(&self, session: i64) -> f64 {
        let mut sumup = 0.0 as f64;

        for i in 0..self.population.len() {
            sumup += self.population[i].estimate(session);
        }

        return sumup/self.population.len() as f64;
    }

    pub fn best_fitness(&self, session: i64) -> f64 {
        let mut best = 0.0 as f64;
        let mut id = 0;

        for i in 0..self.population.len() {
            let val = self.population[i].estimate(session);
            best = best.max(val);
            if best == val {
                id = i;
            }
        }
        return best;
    }

    pub fn best_player(&self, session: i64) -> T {
        let mut best = 0.0 as f64;
        let mut id_best = 0;

        for i in 0..self.population.len() {
            let tmp = best.max(self.population[i].estimate(session));
            if tmp != best {
                best = tmp;
                id_best = i;
            }
        }

        return self.population[id_best].player.clone();
    }

    pub fn estimate(&mut self, session: i64) {
        let individual_estimation = self.population
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
        &mut self, number_of_couple: usize, session: i64
    ) {
        let mut rng = rand::thread_rng();
        let mut extintion = Vec::<usize>::new();
        let mut roulette = vec![0.0 as f64; self.population.len()];
        let mut sumup = 0.0 as f64;

        let individual_estimation = self.population
            .par_iter()
            .enumerate()
            .map(|(i, iter)| (i, iter.player.estimate()))
            .collect::<Vec<(usize, f64)>>();

        for (i, fitness) in individual_estimation {
            // @NOTE: update fitness of current session into each player
            self.population[i].update_fitness(session, fitness);

            // @NOTE: cache fitness for calculating later
            roulette[i] = fitness;
            sumup += fitness;

            // @NOTE: to make some events like mass extinction, we need to perform estimation
            //        who would be removed out of our population
            if (self.extinguish)(&self.population[i], session) {
                extintion.push(i);
            }
        }

        // @NOTE: remove dead player
        if extintion.len() < self.population.len() {
            let percent = extintion.len() as f64 / self.population.len() as f64;

            for i in extintion.into_iter().rev() {
                if rng.gen::<f64>() < percent {
                    self.population.remove(i);
                }
            }
        }

        // @NOTE: force to remove players who have bad quality
        if self.population.len() > self.limit {

            // @NOTE: sort by estimation fitness
            self.population.sort_by(
                |a, b| b.estimate(session).partial_cmp(&a.estimate(session)).unwrap()
            );

            // @NOTE: remove old player
            self.population.truncate(self.population.len() - self.limit);
        }


        for i in 0..self.population.len() {
            roulette[i] /= sumup;
        }

        for i in 1..self.population.len() {
            roulette[i] += roulette[i - 1];
        }

        // @NOTE: try to promote new player
        for _ in 0..number_of_couple {
            // @NOTE: random picks two players who has very lucky
            let f = self.roulette_wheel_selection(&mut roulette, rng.gen::<f64>());
            let m = self.roulette_wheel_selection(&mut roulette, rng.gen::<f64>());

            // @NOTE: add new player
            self.population.push(
                Individual::<T>::new(
                    (self.crossover)(
                        self,
                        &self.population[f].player, f,
                        &self.population[m].player, m,
                        session,
                    ),
                ),
            );
        }
    }

    pub fn fluctuate(&mut self, session: i64, arguments: Vec<Vec<f64>>, mutation_rate: f64) {
        let mut rng = rand::thread_rng();

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
    }

    fn roulette_wheel_selection(
        &self, 
        roulette: &mut Vec<f64>,
        target: f64,
    ) -> usize {
        // @TODO: use binary search to reduce if we have a lot of players
        for i in 0..self.population.len() {
            if target > roulette[i] {
                continue;
            }

            return i;
        }

        return self.population.len() - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestPlayer {
        count: f64,
    }

    impl Player for TestPlayer {
        fn initialize(&mut self) {
            let mut rng = rand::thread_rng();

            self.count = (rng.gen::<f64>() - 0.5) * 1.0;
        }

        fn estimate(&self) -> f64 {
            if self.count < 10.0 {
                1.0/(10.0 - self.count)
            } else {
                1.0/(self.count - 10.0)
            }
        }

        fn gene(&self) -> Vec<f64> {
            vec![self.count]
        }
    }

    fn generate_players() -> Vec<TestPlayer> {
        let mut players: Vec<TestPlayer> = Vec::<TestPlayer>::new();
        for _ in 0..10 {
            players.push(TestPlayer { count: 0.0 });
        }

        return players;
    }

    fn merging_crossover(
        _controller: &Genetic<TestPlayer>,
        father_ctx: &TestPlayer, father_id: usize, 
        mother_ctx: &TestPlayer, mother_id: usize,
        session_id: i64,
    ) -> TestPlayer {
        TestPlayer{ count: (father_ctx.count + mother_ctx.count)/2.0 }
    }

    fn adaptive_crossover(
        controller: &Genetic<TestPlayer>,
        father_ctx: &TestPlayer, father_id: usize, 
        mother_ctx: &TestPlayer, mother_id: usize,
        session_id: i64,
    ) -> TestPlayer {
        let mut rng = rand::thread_rng(); 
        let alpha = rng.gen::<f64>() * (session_id as f64) / (father_ctx.estimate() + mother_ctx.estimate());

        TestPlayer{ count: alpha*father_ctx.count + (1.0 - alpha)*mother_ctx.count }
    }

    fn mutate(
        player: &mut TestPlayer,
        arguments: &Vec<f64>,
        gene: usize,
    ) {
    }

    fn policy(
        player: &Individual<TestPlayer>,
        session_id: i64,
    ) -> bool {
        return false;
    }

    #[test]
    fn test_genetic_algorithm_workflow() {
        let mut genetic = Genetic::<TestPlayer>::new(
            10, 
            adaptive_crossover,
            mutate,
            policy,
        );

        genetic.initialize(generate_players());

        for j in 0..1 {
            for i in 0..100 {
                genetic.evolute(10, j*100 + i);
            }
        }
    }
}
