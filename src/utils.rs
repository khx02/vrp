use rand::seq::IteratorRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::config;
use crate::domain::types::Route;

pub fn temperature(runs: usize, iteration: usize, temperature_factor: i32) -> f64 {
    (((runs as f64) - (iteration as f64)) / (runs as f64)) * (temperature_factor as f64)
}

pub fn steer_towards_best(current_solution: &mut Route, best_so_far: &Route, num_indices: usize) {
    let seed: u64 = config::constant::SEED as u64;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // Pick `num_indices` unique random positions
    let chosen_indices: Vec<usize> =
        (0..current_solution.route.len()).choose_multiple(&mut rng, num_indices);

    for &idx in &chosen_indices {
        let target_value = best_so_far.route[idx]; // What should be at this index
        let current_idx = current_solution
            .route
            .iter()
            .position(|&x| x == target_value)
            .unwrap(); // Find where it is

        // Swap to move `target_value` into the correct position
        current_solution.route.swap(idx, current_idx);
    }
}

pub fn swaps_overlap(a: &(usize, usize), b: &(usize, usize)) -> bool {
    a.0 == b.0 || a.0 == b.1 || a.1 == b.0 || a.1 == b.1
}

pub fn swap_indices(solution: &Route, indices: (usize, usize)) -> Route {
    let mut new_solution = solution.to_owned();
    new_solution.route.swap(indices.0, indices.1);
    new_solution
}
