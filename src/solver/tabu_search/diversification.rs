use crate::domain::types::Route;
use rand::seq::IteratorRandom;
use rand_chacha::ChaCha8Rng;

/// Perform rollback to diversify search by returning to a past high-quality solution
pub fn perform_rollback(
    saved_solutions: &[Route],
    len_tabu_list: usize,
    next_solution: &mut Route,
    best_so_far: &Route,
) -> Route {
    let needed = len_tabu_list.saturating_mul(4);
    if saved_solutions.len() < needed + 1 {
        return next_solution.clone();
    }

    let mut operating_solution = next_solution.clone();
    let mut overall_reduction = 0.0;

    let start = saved_solutions.len() - needed;
    for ind in (start + 1)..saved_solutions.len() {
        overall_reduction += saved_solutions[ind - 1].fitness - saved_solutions[ind].fitness;
    }

    if overall_reduction > 0.0 && next_solution.route != best_so_far.route {
        operating_solution = best_so_far.clone();
    }
    operating_solution
}

/// Apply final mutation: reverse segments and swap elements
pub fn final_mutation(next_solution: &mut Route, rng: &mut ChaCha8Rng) {
    let n = next_solution.route.len();
    if n < 2 {
        return;
    }

    // Reverse a random segment
    let mut pair: Vec<usize> = (0..n).choose_multiple(rng, 2);
    pair.sort_unstable();
    let (a, b) = (pair[0], pair[1]);
    next_solution.route[a..=b].reverse();

    // Apply 3-opt style swaps
    if n >= 3 {
        let mut triple: Vec<usize> = (0..n).choose_multiple(rng, 3);
        triple.sort_unstable();
        let (x, y, z) = (triple[0], triple[1], triple[2]);
        next_solution.route.swap(x, y);
        next_solution.route.swap(y, z);
    }
}
