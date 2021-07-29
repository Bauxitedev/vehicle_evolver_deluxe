use crate::{
    plugins::genetics::{GlobalFitnessMap, SimulationParams},
    vehicle::Vehicle,
};

use rand::seq::IteratorRandom;
use rand::Rng;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

#[derive(new, Debug)]
pub struct GenerationalStatistics {
    pub avg_fitness: f64,
    pub max_fitness: f64,
}
pub struct GeneticsSimulator {
    population: Pop,
    generational_statistics: Vec<GenerationalStatistics>,
}
pub type Pop = Vec<(Vehicle, Option<i64>)>; //Fitness

impl GeneticsSimulator {
    pub fn new(population_size: usize) -> Self {
        assert!(
            population_size % 2 == 0,
            "population size wasn't even ({})",
            population_size
        );
        let mut population = vec![];
        for _ in 0..population_size {
            population.push((Vehicle::new(), None));
        }
        GeneticsSimulator {
            population,
            generational_statistics: vec![],
        }
    }

    pub fn get_generational_statistics(&self) -> &Vec<GenerationalStatistics> {
        &self.generational_statistics
    }

    pub fn get_population(&self) -> &Pop {
        &self.population
    }

    pub fn get_population_vehicles(&self) -> Vec<Vehicle> {
        self.population.iter().map(|(v, _)| v.clone()).collect()
    }

    pub fn fill_in_fitness(&mut self, map: &GlobalFitnessMap) {
        info!(
            "filling in fitness in the simulator: we have {} entries to pick from",
            map.len()
        );
        for (vehicle, fitness) in &mut self.population {
            let old_fitness = *fitness;
            *fitness = map.get(vehicle).map(|x| *x);
            trace!(
                "fitness went from {:?} to {:?} (found it? {})",
                old_fitness,
                fitness,
                map.contains_key(vehicle)
            );
        }
    }

    pub fn avg_fitness(&self) -> f64 {
        self.get_population()
            .iter()
            .map(|(_, y)| y.expect("can't calculate avg fitness: missing fitness"))
            .sum::<i64>() as f64
            / (self.get_population().len() as f64)
    }

    pub fn max_fitness(&self) -> i64 {
        self.get_population()
            .iter()
            .map(|(_, y)| y.expect("can't calculate max fitness: missing fitness"))
            .max()
            .expect("empty population")
    }

    pub fn step(&mut self, params: &SimulationParams) {
        //1. evaluate fitness
        //2. variation (crossover and then mutation)
        //3. rank based selection (truncation or tournament) -> they become the new parents

        //NOTE: gonna apply a slight tweak to make this easier
        //we're gonna do windows instead of chunks for crossover
        //and we're gonna make it so parents don't survive to the next generation

        let tournament_k = params.tournament_k as usize;

        let avg_fitness = self.avg_fitness();
        let max_fitness = self.max_fitness() as f64;

        let stats = GenerationalStatistics::new(avg_fitness, max_fitness);
        self.generational_statistics.push(stats);

        let new_parents = self.tournament_selection(tournament_k, self.population.len());
        let mut children = self.crossover(&new_parents);

        assert_eq!(children.len(), (new_parents).len());

        for (child, _) in &mut children {
            child.mutate(params.mutation_amount as usize);
        }

        self.population = children;
    }

    pub fn overwrite_population(&mut self, pop: Vec<Vehicle>) {
        self.population = pop.into_iter().map(|vehicle| (vehicle, None)).collect();
    }

    pub fn tournament_selection(&self, k: usize, n: usize) -> Pop {
        //Select best individual from k randomly selected individuals.
        //Hold n tournaments to get n new individuals.

        //low k = small selection pressure
        //high k = strong selection pressure, but homogenizes the population (many identical genotypes)

        assert!(k > 1);
        assert!(k <= n);
        let mut rng = rand::thread_rng();

        let mut result = vec![];
        for _ in 0..n {
            let tournament = self.population.iter().choose_multiple(&mut rng, k);
            let winner = tournament
                .into_iter()
                .max_by_key(|(_, x)| x.expect("you didn't calculate fitness for every vehicle yet"))
                .unwrap();

            result.push(winner.clone());
        }

        info!(
            "doing tournament selection on {} vehicles with k={} n={}",
            self.population.len(),
            k,
            n
        );
        info!("the winners are: {:?}", result);

        result
    }

    #[allow(dead_code)]
    pub fn print_pop(&self) {
        info!("{:?}", self.population);
    }

    pub fn crossover(&mut self, parents: &Pop) -> Pop {
        let mut children = vec![];
        let mut rng = rand::thread_rng();

        let pop_size = parents.len();
        assert!(
            pop_size % 2 == 0,
            "population size wasn't even ({})",
            pop_size
        );

        let pop_subset = parents.to_owned();

        for parents in pop_subset.windows(2) {
            let father = &parents[0];
            let mother = &parents[1];

            info!(
                "crossbreeding with parents with fitness {:?} and {:?}",
                father.1, mother.1
            );

            let crossover_point = rng.gen_range(1..crate::vehicle::VEHICLE_SHAPE.1);

            let (brother, sister) = father.0.one_point_crossover(&mother.0, crossover_point);

            children.push(brother);
            children.push(sister);
        }

        children
            .into_iter()
            .map(|x| (x, None))
            .choose_multiple(&mut rng, parents.len()) //Pick from all brothers and sisters randomly
    }
}
