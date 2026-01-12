const RUNS: usize = 20000;
const LOCATION_COUNT: usize = 76;

// Module declarations
mod api;
mod database;
mod evaluation;
mod phases;
mod setup;
mod test;
mod utils;

// Import functions from modules
pub use crate::phases::core_logic::{
    choose_best_candidate, final_mutation, find_neighbours, insert_and_adjust_tabu_list,
    perform_rollback,
};
use crate::phases::types::*;
use database::sqlx::db_connection;
use evaluation::eval_funcs::*;
use rand_chacha::ChaCha8Rng;
use setup::init::setup;
use test::get_random_inputs;
use utils::{steer_towards_best, temperature};

// External crate imports
use colored::*;
use csv::Writer;
use rand::{thread_rng, Rng, SeedableRng};
use std::collections::BinaryHeap;
use std::{cmp::max, collections::VecDeque, error::Error};
use tracing::{debug, info, span, trace, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const PENALTY_VALUE: u64 = 20;

#[tokio::main]
#[tracing::instrument(name = "VRP Solver", level = "info")]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialise tracing_subscriber for better logging/debugging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env()) // Reads RUST_LOG
        .with(
            fmt::layer()
                .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
                .pretty(), // or .compact() if you want less verbose
        )
        .init();

    let db_pool = db_connection().await?;

    info!(
        "Starting VRP solver with {} locations and {} iterations",
        LOCATION_COUNT, RUNS
    );

    // INPUT
    let (locations, mut loc_cap, mut vehicle_cap) = get_random_inputs(LOCATION_COUNT, "207224");

    let num_of_trucks: usize = vehicle_cap.len();
    vehicle_cap.sort_unstable_by(|a, b| b.cmp(a));
    if num_of_trucks > 1 {
        loc_cap.splice(0..0, std::iter::repeat(0).take(num_of_trucks - 2));
    }

    // SETUP
    let mut no_seed_rng = thread_rng();
    let (problem_instance, initial_solution) = {
        let span = span!(Level::INFO, "setup");
        let _guard = span.enter();
        let result = setup(
            num_of_trucks,
            &mut vehicle_cap,
            &locations,
            &mut loc_cap,
            PENALTY_VALUE,
            "osrm",
            None,
            db_pool,
        )
        .await;
        result
    };

    // === SEARCH STATE ===
    let mut current_solution = initial_solution.clone();
    let mut best_so_far: Route = initial_solution.clone();
    let mut best_so_far_iteration = 0;

    let mut saved_solutions: Vec<Route> = vec![];
    let aspiration_threshold = 20.0;
    let mut parent_swap: (usize, usize) =
        (current_solution.route.len(), current_solution.route.len());

    // Stagnation tracking
    let mut stagnation = 0;
    let mut max_stagnation = 0;
    let scaling_factor = if locations.len() < 50 { 15.0 } else { 9.0 };
    let max_no_improvement = max(
        300,
        (scaling_factor * (locations.len() as f64).powf(1.33)) as usize,
    );
    let mut temperature_factor = 1;

    // Early termination snapshot
    let mut ended_early_value = 0.0;
    let mut has_ended = false;
    let mut ended_early_iteration = 0;

    let mut rng = ChaCha8Rng::seed_from_u64(67);

    // Tabu list
    let mut len_tabu_list = 20;
    let tl_upper_bound_len = 29;
    let tl_lower_bound_len = 11;
    let mut tabu_list: VecDeque<(usize, usize)> = VecDeque::new();

    info!("INITIAL SOLUTION:");
    print_solution(&initial_solution, &problem_instance);

    let mut c1 = 0;
    let mut c2 = 0;
    let mut c3 = 0;
    let mut c4 = 0;
    let mut best_so_far_updates: Vec<(usize, f64)> = vec![];

    // === MAIN OPTIMIZATION LOOP ===
    let loop_span = span!(Level::INFO, "main_search_loop", total_iterations = RUNS);
    let _loop_guard = loop_span.enter();

    for iteration in 1..=RUNS {
        let iter_span = span!(Level::DEBUG, "iteration", iter = iteration);
        let _iter_guard = iter_span.enter();

        debug!("=== Iteration {} ===", iteration);

        saved_solutions.push(current_solution.clone());

        // PHASE 1: Find neighbours
        let swap_candidates_ind: Vec<(f64, (usize, usize))> = {
            let span = span!(Level::DEBUG, "find_neighbours");
            let _g = span.enter();
            find_neighbours(&current_solution, &problem_instance)
        };

        // PHASE 2: Choose best candidate
        let chosen_solution = choose_best_candidate(
            &swap_candidates_ind,
            &tabu_list,
            &best_so_far,
            aspiration_threshold,
            &parent_swap,
        );

        debug!(
            "chosen swap: {:.2}, {:?}",
            chosen_solution.0, chosen_solution.1
        );

        let mut final_neighbour = Route {
            route: current_solution.route.clone(),
            fitness: chosen_solution.0,
        };
        final_neighbour
            .route
            .swap(chosen_solution.1 .0, chosen_solution.1 .1);

        insert_and_adjust_tabu_list(&mut tabu_list, chosen_solution.1, len_tabu_list);

        if final_neighbour.fitness < best_so_far.fitness {
            best_so_far = final_neighbour.clone();
            best_so_far_iteration = iteration;
            best_so_far_updates.push((iteration, final_neighbour.fitness));
            info!(
                "New best at iteration {}: fitness = {:.2}",
                iteration, best_so_far.fitness
            );
        }

        parent_swap = chosen_solution.1;

        // PHASE 3: Diversification / Mutation
        let temp = temperature(RUNS, iteration, temperature_factor);
        let mut next_solution = final_neighbour;

        let mutate_to_best_check = iteration % 50;
        let mutate_steer_best_check = iteration % 40;
        let mutate_tabu_len_check = iteration % 20;

        if no_seed_rng.gen::<f64>() * no_seed_rng.gen_range(0.3..0.6)
            <= temp * no_seed_rng.gen_range(0.9..1.0)
            && mutate_to_best_check == 0
            && saved_solutions.len() > (len_tabu_list * 4)
        {
            c1 += 1;
            next_solution = perform_rollback(
                &saved_solutions,
                len_tabu_list,
                &mut next_solution,
                &best_so_far,
            );
        } else if mutate_steer_best_check == 0 {
            c2 += 1;
            let num_to_change =
                ((next_solution.route.len() as f64) * temp * no_seed_rng.gen::<f64>()).ceil()
                    as usize;
            steer_towards_best(&mut next_solution, &best_so_far, num_to_change);
        }

        if mutate_tabu_len_check == 0 && tl_lower_bound_len < tl_upper_bound_len {
            c3 += 1;
            len_tabu_list = no_seed_rng.gen_range(tl_lower_bound_len..tl_upper_bound_len);
        }

        if no_seed_rng.gen::<f64>() * no_seed_rng.gen_range(0.4..0.6)
            <= temp * no_seed_rng.gen_range(0.8..1.0)
        {
            final_mutation(&mut current_solution, &mut rng);
            c4 += 1;
        }

        // Repair infeasible solutions
        next_solution.fitness = find_fitness(
            &next_solution,
            &problem_instance.penalty_value,
            &num_of_trucks,
            &problem_instance.vehicle_capacities,
            &problem_instance.distance_matrix,
        );
        let next_dist = find_distance(&next_solution, &problem_instance.distance_matrix);

        if next_solution.fitness > next_dist {
            info!("DEFECT - Repairing infeasible solution");
            print_solution(&next_solution, &problem_instance);
            next_solution = anls_destroy_and_recreate(&mut next_solution, &problem_instance);
        }

        if next_solution.fitness < best_so_far.fitness {
            best_so_far = next_solution.clone();
            best_so_far_iteration = iteration;
            best_so_far_updates.push((iteration, next_solution.fitness));
            info!(
                "New best at iteration {}: fitness = {:.2}",
                iteration, best_so_far.fitness
            );
        }

        // Stagnation logic
        if best_so_far_iteration != iteration {
            stagnation += 1;
            if stagnation >= max_no_improvement && !has_ended {
                info!("ENDED EARLY AT ITERATION: {}", iteration);
                ended_early_value = best_so_far.fitness;
                has_ended = true;
                ended_early_iteration = iteration;
            } else if stagnation >= max_no_improvement / 2 && !has_ended {
                temperature_factor = 2;
            }
        } else {
            max_stagnation = max(stagnation, max_stagnation);
            stagnation = 0;
            temperature_factor = 1;
        }

        current_solution = next_solution;

        trace!("Current solution at end of iteration:");
        print_solution(&current_solution, &problem_instance);
    }

    // === FINAL REPORTING ===
    info!(
        "Optimization complete. Best solution found at iteration {}",
        best_so_far_iteration
    );
    print_solution(&best_so_far, &problem_instance);

    info!("Max Stagnation: {}", max_stagnation);
    info!("Early end triggered: {}", has_ended);
    if has_ended {
        info!(
            "Ended early at iteration {} with fitness {:.2}",
            ended_early_iteration, ended_early_value
        );
        info!(
            "Improvement after early trigger: {:.2} ({:.2}%)",
            ended_early_value - best_so_far.fitness,
            ((ended_early_value - best_so_far.fitness) / ended_early_value) * 100.0
        );
    }

    info!(
        "Mutation counts - rollback: {}, steer: {}, tabu_len: {}, final: {}",
        c1, c2, c3, c4
    );

    save_to_csv(
        &best_so_far_updates,
        ended_early_iteration,
        "best_so_far.csv",
    )?;

    Ok(())
}

fn save_to_csv(
    best_so_far_updates: &Vec<(usize, f64)>,
    ended_early_iteration: usize,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path(filename)?;

    // Write header
    wtr.write_record(&["iteration", "new_best_so_far", "ended_early_iteration"])?;

    // Write data
    for (iteration, value) in best_so_far_updates {
        // let marker = if *iteration == ended_early_iteration { "1" } else { "0" };
        wtr.write_record(&[
            iteration.to_string(),
            value.to_string(),
            ended_early_iteration.to_string(),
        ])?;
    }

    wtr.flush()?; // Ensure data is written
    Ok(())
}

fn anls_destroy_and_recreate(solution: &mut Route, pi: &ProblemInstance) -> Route {
    // let mut index_dict: Vec<usize> = vec![0; solution.route.len()];
    // for ind in 0..solution.route.len(){
    //     let loc_index = solution.route[ind].index;
    //     index_dict[loc_index] = ind;
    // }

    let mut trucks = trucks_by_excess(solution, pi);
    let mut destroyed_locations_max_heap = BinaryHeap::new();
    for truck in &mut trucks {
        if truck.excess <= 0 {
            break;
        }

        while truck.excess > 0 {
            let destroyed_location = truck
                .route
                .pop()
                .expect("Error: Tried to pop from an empty route!");
            truck.load -= destroyed_location.demand;
            truck.excess -= destroyed_location.demand as i64;
            destroyed_locations_max_heap.push(destroyed_location);
        }
    }

    // re-create the route from the destroyed locations
    for truck in trucks.iter_mut().rev() {
        if destroyed_locations_max_heap.is_empty() || truck.excess > 0 {
            break;
        }

        // reinsert them in order of highest demand first into trucks, starting with the one with most space left
        while truck.excess < 0 &&                                                               // while there is still space left
                !destroyed_locations_max_heap.is_empty() &&
                truck.excess + destroyed_locations_max_heap.peek().unwrap().demand as i64 <= 0
        // and addition of the new loc wont send it over
        {
            truck
                .route
                .push(destroyed_locations_max_heap.pop().unwrap())
        }
    }

    if !destroyed_locations_max_heap.is_empty() {
        if let Some(lowest_excess_truck) = trucks.iter_mut().min_by_key(|t| t.excess) {
            // Move all elements from destroyed_locations into lowest_excess_truck.route
            lowest_excess_truck
                .route
                .extend(destroyed_locations_max_heap.drain());
        }
    }

    let recreated_solution = recreate_route_from_trucks(&mut trucks, pi);
    recreated_solution
}

fn recreate_route_from_trucks(trucks: &mut Vec<Truck>, pi: &ProblemInstance) -> Route {
    let mut recreated_route: Vec<Location> = vec![];
    let mut partition_counter = 0;
    for (i, truck) in trucks.iter().enumerate() {
        recreated_route.extend(truck.route.clone());

        // Only append partition if this is NOT the last truck
        if i < trucks.len() - 1 {
            recreated_route.push(Location {
                index: partition_counter,
                demand: 0,
                is_warehouse: true,
            });
            partition_counter += 1;
        }
    }

    let mut recreated_solution = Route {
        route: recreated_route,
        fitness: 0.0,
    };

    recreated_solution.fitness = find_fitness(
        &recreated_solution,
        &pi.penalty_value,
        &pi.num_of_trucks,
        &pi.vehicle_capacities,
        &pi.distance_matrix,
    );
    recreated_solution
}

fn print_solution(solution: &Route, problem_instance: &ProblemInstance) {
    let partition = partition_solution(solution, &problem_instance.vehicle_capacities);

    let dist = find_distance(solution, &problem_instance.distance_matrix);
    let fitness = find_fitness(
        solution,
        &problem_instance.penalty_value,
        &problem_instance.num_of_trucks,
        &problem_instance.vehicle_capacities,
        &problem_instance.distance_matrix,
    );
    let pen = penalty(
        solution,
        &problem_instance.penalty_value,
        &problem_instance.num_of_trucks,
        &problem_instance.vehicle_capacities,
    );

    if pen > 0.0 {
        // println!("{}", format!("Distance: {:.2}, {}", dist, format!("Fitness: {:.2}, Penalty: {:.2}", fitness, pen).red()));
        println!(
            "Distance: {:.2}, {}",
            dist,
            format!("Fitness: {:.2}, Penalty: {:.2}", fitness, pen).red()
        );
    } else {
        // println!("{} , {}", format!("Distance: {:.2}", dist).green(), format!("Fitness: {:.2}, Penalty: {:.2}", fitness, pen));
        println!(
            "{} , {}",
            format_args!("Distance: {:.2}", dist).to_string().green(),
            format_args!("Fitness: {:.2}, Penalty: {:.2}", fitness, pen)
        );
    }

    print_location_array(solution);
    for (route, load, capacity) in partition {
        println!("{} / {} : {:?}", load, capacity, route)
    }
}

fn partition_solution(solution: &Route, vehicle_capacity: &[u64]) -> Vec<(Vec<usize>, u64, u64)> {
    let mut route_partition: Vec<(Vec<usize>, u64)> = vec![];
    let mut temp_partition: Vec<usize> = vec![];
    let mut temp_load = 0;
    for loc in &solution.route {
        // println!("{}", loc.is_warehouse);
        if !loc.is_warehouse {
            temp_partition.push(loc.index);
            temp_load += loc.demand
        } else {
            // println!("{:?}", temp_partition);
            route_partition.push((temp_partition, temp_load));
            temp_partition = vec![];
            temp_load = 0;
        }
    }
    route_partition.push((temp_partition, temp_load));
    // println!("route partition: {:?}", route_partition);

    route_partition.sort_by_key(|&(_, value)| std::cmp::Reverse(value));
    let mut partition: Vec<(Vec<usize>, u64, u64)> = vec![];

    for (ind, (r, load)) in route_partition.iter().enumerate() {
        partition.push((r.clone(), *load, vehicle_capacity[ind])); // Clone to avoid ownership issues
    }
    partition
}

fn print_location_array(solution: &Route) {
    let mut loc_indices: Vec<usize> = vec![];
    for loc in &solution.route {
        loc_indices.push(loc.index);
    }
    println!("solution route: {:?}", loc_indices)
}
