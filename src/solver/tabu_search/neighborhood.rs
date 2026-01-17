use rayon::prelude::*;

use crate::domain::types::{ProblemInstance, Route};
use crate::evaluation::fitness::find_fitness;

/// Generate and score neighbour solutions by swapping two positions.
pub fn find_neighbours(
    current_solution: &Route,
    problem_instance: &ProblemInstance,
) -> Vec<(f64, (usize, usize))> {
    let n = current_solution.route.len();

    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
        .collect();

    let mut swap_candidates_ind: Vec<(f64, (usize, usize))> = pairs
        .par_iter()
        .map(|&(i, j)| {
            let mut new_route = current_solution.route.clone();
            new_route.swap(i, j);

            let new_solution = Route {
                route: new_route,
                fitness: 0.0,
            };

            let penalized_dist = find_fitness(
                &new_solution,
                &problem_instance.penalty_value,
                &problem_instance.num_of_trucks,
                &problem_instance.vehicle_capacities,
                &problem_instance.distance_matrix,
            );

            (penalized_dist, (i, j))
        })
        .collect();

    swap_candidates_ind.par_sort_by(|a, b| a.0.total_cmp(&b.0));
    swap_candidates_ind
}