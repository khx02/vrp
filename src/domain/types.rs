use rand::thread_rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use std::collections::VecDeque;

use crate::config::constant::SEED;

#[derive(Debug, Clone)]
pub struct Route {
    pub route: Vec<Location>,
    pub fitness: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub index: usize,
    pub demand: u64,
    pub is_warehouse: bool,
}

// Order by descending demand so max-heaps pick larger loads first.
impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.demand.cmp(&self.demand)
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct Truck {
    pub load: u64,
    pub capacity: u64,
    pub excess: i64,
    pub ending_warehouse: Option<usize>,
    pub route: Vec<Location>,
}

#[derive(Debug, Clone)]
pub struct ProblemInstance {
    pub locations_string: Vec<String>,
    pub distance_matrix: Vec<Vec<f64>>,
    pub vehicle_capacities: Vec<u64>,
    pub location_demands: Vec<u64>,
    pub num_of_trucks: usize,
    pub penalty_value: u64,
}

#[derive(Debug, Deserialize)]
pub struct MRTLocation {
    #[serde(rename = "Possible Locations")]
    pub possible_locations: Vec<LocationData>,
}

#[derive(Debug, Deserialize)]
pub struct LocationData {
    #[serde(rename = "POSTAL")]
    pub postal: String,
}

/// Encapsulates all mutable state during tabu search iteration
#[derive(Debug)]
pub struct SearchState {
    pub current_solution: Route,
    pub best_so_far: Route,
    pub best_so_far_iteration: usize,
    pub saved_solutions: Vec<Route>,
    pub parent_swap: (usize, usize),
    pub stagnation: usize,
    pub max_stagnation: usize,
    pub temperature_factor: i32,
    pub ended_early_value: f64,
    pub has_ended: bool,
    pub ended_early_iteration: usize,
    pub rng: ChaCha8Rng,
    pub len_tabu_list: usize,
    pub tabu_list: VecDeque<(usize, usize)>,
    pub best_so_far_updates: Vec<(usize, f64)>,
    pub no_seed_rng: rand::rngs::ThreadRng,
    // Mutation counters
    pub c1: usize,
    pub c2: usize,
    pub c3: usize,
    pub c4: usize,
}

impl SearchState {
    /// Initialize search state from initial solution
    pub fn new(initial_solution: Route) -> Self {
        let route_len = initial_solution.route.len();
        SearchState {
            current_solution: initial_solution.clone(),
            best_so_far: initial_solution,
            best_so_far_iteration: 0,
            saved_solutions: vec![],
            parent_swap: (route_len, route_len),
            stagnation: 0,
            max_stagnation: 0,
            temperature_factor: 1,
            ended_early_value: 0.0,
            has_ended: false,
            ended_early_iteration: 0,
            rng: ChaCha8Rng::seed_from_u64(SEED as u64),
            len_tabu_list: 20,
            tabu_list: VecDeque::new(),
            best_so_far_updates: vec![],
            no_seed_rng: thread_rng(),
            c1: 0,
            c2: 0,
            c3: 0,
            c4: 0,
        }
    }
}
