use rand::Rng;

#[derive(Debug)]
struct Individual<T> {
    player:  T,
    session: i64,
    fitness: f64,
}

pub type CrossoverCallback<T> = fn(&Genetic<T>, &T, usize, &T, usize, i64) -> T;

pub trait Player {
    fn initialize(&mut self);
    fn estimate(&self) -> f64;
}

#[derive(Debug)]
pub struct Genetic<T>{
    population: Vec<Individual<T>>,
    limit:      usize,
    crossover:  CrossoverCallback<T>,
}

impl <T: Player + Clone + std::fmt::Debug> Individual<T> {
    pub fn new(player: T) -> Self {
        Self{
            player: player,
            fitness: 0.0,
            session: -1,
        }
    }

    pub fn estimate(&self, session: i64) -> f64 {
        if session != self.session {
            return 0.0;
        }

        return self.fitness;
    }

    pub fn estimate_mut(&mut self, session: i64) -> f64 {
        if self.session != session && self.session < session {
            self.fitness = self.player.estimate();
            self.session = session;
        }

        return self.fitness
    }

    pub fn initialize(&mut self) {
        self.player.initialize();
    }
}

impl <T: Player + Clone + std::fmt::Debug> Genetic<T> {
    pub fn new(
        players: Vec<T>,
        limit: usize,
        crossover: CrossoverCallback<T>,
    ) -> Self {
        let mut population = Vec::<Individual<T>>::new();

        for player in players {
            population.push(Individual::<T>::new(player));
        }

        Self {
            population:   population,
            limit:     limit,
            crossover: crossover,
        }
    }

    pub fn initialize(&mut self) {
        for player in &mut self.population {
            player.initialize();
        }
    }

    pub fn evolute(&mut self, number_of_couple: usize, session: i64) {
        let mut rng = rand::thread_rng();
        let mut roulette = vec![0.0 as f64; self.population.len()];
        let mut sumup = 0.0 as f64;

        for i in 0..self.population.len() {
            roulette[i] = self.population[i].estimate_mut(session);
            sumup += roulette[i];
        }

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
    }

    fn generate_players() -> Vec<TestPlayer> {
        let mut players: Vec<TestPlayer> = Vec::<TestPlayer>::new();
        for _ in 0..10 {
            players.push(TestPlayer { count: 0.0 });
        }

        return players;
    }

    fn merging_crossover(
        controller: &Genetic<TestPlayer>,
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

    #[test]
    fn test_genetic_algorithm_workflow() {
        let mut genetic = Genetic::<TestPlayer>::new(
            generate_players(), 
            10, 
            adaptive_crossover,
        );

        genetic.initialize();

        for j in 0..1 {
            for i in 0..100 {
                genetic.evolute(10, j*100 + i);
            }
        }
    }
}
