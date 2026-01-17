use rand::seq::IteratorRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use crate::evaluation::eval_funcs::*;
use crate::phases::phases_types::*;
use crate::utils::swaps_overlap;
use crate::VecDeque;

/// Generate all possible neighbour solutions by swapping two positions
/// in the current route, evaluate their fitness, and return them sorted.
///
/// Returns:
///   Vec of (fitness, (i, j)) where (i, j) is the swapped index pair
pub fn find_neighbours(
    current_solution: &Route,
    problem_instance: &ProblemInstance,
) -> Vec<(f64, (usize, usize))> {
    // Number of locations in the current route
    let n = current_solution.route.len();

    // Step 1: Precompute all index pairs (i, j) where i < j
    // Example: for n = 4 -> (0,1), (0,2), (0,3), (1,2), (1,3), (2,3)
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
        .collect();

    // Step 2: In parallel, evaluate the fitness of each swapped solution
    let mut swap_candidates_ind: Vec<(f64, (usize, usize))> = pairs
        .par_iter() // Parallel iterator over all pairs
        .map(|&(i, j)| {
            // Clone just the route Vec (not the whole Route)
            let mut new_route = current_solution.route.clone();

            // Swap the two locations at indices i and j
            new_route.swap(i, j);

            // Build a new Route object with the modified order
            // Fitness is set to 0.0 because we'll compute it fresh below
            let new_solution = Route {
                route: new_route,
                fitness: 0.0,
            };

            // Step 2a: Compute penalized fitness for this new route
            // Takes into account distance + capacity penalties
            let penalized_dist = find_fitness(
                &new_solution,
                &problem_instance.penalty_value,
                &problem_instance.num_of_trucks,
                &problem_instance.vehicle_capacities,
                &problem_instance.distance_matrix,
            );

            // Return a tuple of (fitness, indices swapped)
            (penalized_dist, (i, j))
        })
        .collect();

    // Step 3: Sort neighbours by fitness (ascending = better first)
    // total_cmp is used instead of partial_cmp to handle NaN/Inf safely
    swap_candidates_ind.par_sort_by(|a, b| a.0.total_cmp(&b.0));

    // Step 4: Return the full sorted neighbour list
    swap_candidates_ind
}

// phase 3
pub fn perform_rollback(
    saved_solutions: &[Route],
    len_tabu_list: usize,
    next_solution: &mut Route,
    best_so_far: &Route,
) -> Route {
    // If we don't have enough history, just return current
    let needed = len_tabu_list.saturating_mul(4);
    if saved_solutions.len() < needed + 1 {
        return next_solution.clone();
    }

    let mut operating_solution = next_solution.clone();
    let mut overall_reduction = 0.0;

    // Sum fitness differences over the last `needed` transitions
    let start = saved_solutions.len() - needed;
    for ind in (start + 1)..saved_solutions.len() {
        overall_reduction += saved_solutions[ind - 1].fitness - saved_solutions[ind].fitness;
    }

    // If the trend is worsening AND we're not already at best, jump back
    if overall_reduction > 0.0 && next_solution.route != best_so_far.route {
        operating_solution = best_so_far.clone();
    }
    operating_solution
}

pub fn final_mutation(next_solution: &mut Route, rng: &mut ChaCha8Rng) {
    let n = next_solution.route.len();
    if n < 2 {
        return;
    }

    // Reverse a random sublist (choose 2 distinct indices)
    let mut pair: Vec<usize> = (0..n).choose_multiple(rng, 2);
    pair.sort_unstable();
    let (a, b) = (pair[0], pair[1]);
    next_solution.route[a..=b].reverse();

    // Optionally do a 3-swap if there are 3+ nodes
    if n >= 3 {
        let mut triple: Vec<usize> = (0..n).choose_multiple(rng, 3);
        triple.sort_unstable(); // keep swaps well-defined
        let (x, y, z) = (triple[0], triple[1], triple[2]);
        next_solution.route.swap(x, y);
        next_solution.route.swap(y, z);
    }
}

pub fn choose_best_candidate(
    swap_candidates_ind: &[(f64, (usize, usize))],
    // tabu_list: &VecDeque<Route>,
    tabu_list: &VecDeque<(usize, usize)>,
    best_so_far: &Route,
    aspiration_threshold: f64,
    parent_swap: &(usize, usize),
) -> (f64, (usize, usize)) {
    // extra code in case swap_candidates_ind is empty

    let mut chosen_solution = if !swap_candidates_ind.is_empty() {
        swap_candidates_ind[0]
    } else {
        (0.0, (0, 0))
    };

    // === Aspiration Criteria & Tabu List Handling ===

    // Normalise the candidate swap pair (always store i<j so (1,3) == (3,1))
    let cand_pair = {
        let (i, j) = chosen_solution.1;
        if i < j {
            (i, j)
        } else {
            (j, i)
        }
    };

    // Check if this swap is tabu (recently performed)
    if tabu_list.contains(&cand_pair) {
        // Candidate move is tabu.

        // Aspiration criterion: allow tabu move if it's close enough to best fitness
        // AND not the same as the immediate parent swap (to avoid trivial undo).
        if (best_so_far.fitness - aspiration_threshold..=best_so_far.fitness + aspiration_threshold)
            .contains(&chosen_solution.0)
            && !swaps_overlap(&chosen_solution.1, parent_swap)
        {
            // Accept candidate anyway due to aspiration.
        } else {
            // Otherwise, search for the best neighbour that is not tabu.
            chosen_solution = swap_candidates_ind
                .iter()
                .find(|sol| {
                    // Normalise the neighbour's swap too
                    let (i, j) = sol.1;
                    let pair = if i < j { (i, j) } else { (j, i) };

                    // Only accept if not tabu and not overlapping with parent
                    !tabu_list.contains(&pair) && !swaps_overlap(&sol.1, parent_swap)
                })
                .copied()
                // If all moves are tabu, keep the original candidate
                .unwrap_or(chosen_solution);
        }
    }

    chosen_solution
}

/// Insert a new swap move into the tabu list and keep it within the max length.
///
/// - `tabu_list`: queue of recent swaps, each stored as (i, j) with i < j.
/// - `swap_move`: the move (i, j) just applied in this iteration.
/// - `len_tabu_list`: maximum tabu tenure (queue length).
pub fn insert_and_adjust_tabu_list(
    tabu_list: &mut VecDeque<(usize, usize)>,
    swap_move: (usize, usize),
    len_tabu_list: usize,
) {
    // Normalise the swap so (3,1) == (1,3)
    let pair = if swap_move.0 < swap_move.1 {
        swap_move
    } else {
        (swap_move.1, swap_move.0)
    };

    // Add the move at the front of the deque
    tabu_list.push_front(pair);

    // Trim the deque if it exceeds the allowed length
    while tabu_list.len() > len_tabu_list {
        tabu_list.pop_back();
    }
}
