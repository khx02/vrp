use std::cmp::max;
use std::error::Error;

use csv::Writer;
use rand::Rng;
use tracing::{debug, info, span, trace, warn, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::config::constant::{DISTANCE_PROVIDER, LOCATION_COUNT, PENALTY_VALUE, RUNS, WAREHOUSE};
use crate::database::sqlx::db_connection;
use crate::domain::types::{ProblemInstance, Route, SearchState};
use crate::evaluation::fitness::{find_distance, find_fitness};
use crate::evaluation::penalty::penalty;
use crate::fixtures::data_generator::generate_random_inputs;
use crate::setup::init::setup;
use crate::solver::tabu_search::diversification::{final_mutation, perform_rollback};
use crate::solver::tabu_search::repair::anls_destroy_and_recreate;
use crate::utils::{steer_towards_best, temperature};
use dotenv::dotenv;
use std::env;

use super::neighbourhood::find_neighbours;
use super::tabu::{choose_best_candidate, insert_and_adjust_tabu_list};

/// Initialize tracing and environment
fn init_tracing_and_env() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            fmt::layer()
                .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
                .pretty(),
        )
        .init();

    dotenv().ok();
    Ok(())
}

/// Load Google API key if DISTANCE_PROVIDER is set to "google"
fn load_google_api_key() -> Result<Option<String>, Box<dyn Error>> {
    if DISTANCE_PROVIDER == "google" {
        match env::var("GOOGLE_API_KEY") {
            Ok(key) => {
                info!("Loaded Google Maps API key from .env");
                Ok(Some(key))
            }
            Err(_) => {
                eprintln!(
                    "Error: DISTANCE_PROVIDER is 'google' but GOOGLE_API_KEY not found in .env"
                );
                Err("Missing GOOGLE_API_KEY in .env".into())
            }
        }
    } else {
        Ok(None)
    }
}

/// Process input locations and capacities (sort vehicles, splice dummy warehouses)
fn process_inputs(
    mut vehicle_cap: Vec<u64>,
    mut location_capacities: Vec<u64>,
    num_of_trucks: usize,
) -> (Vec<u64>, Vec<u64>) {
    vehicle_cap.sort_unstable_by(|a, b| b.cmp(a));
    debug!("Location capacities: {:?}", location_capacities);

    if num_of_trucks > 1 {
        location_capacities.splice(0..0, std::iter::repeat(0).take(num_of_trucks - 2));
    }
    debug!(
        "Location capacities after splicing: {:?}",
        location_capacities
    );

    (vehicle_cap, location_capacities)
}

/// Calculate maximum iterations without improvement based on problem size
fn calculate_max_no_improvement(locations_len: usize) -> usize {
    let scaling_factor = if locations_len < 50 { 15.0 } else { 9.0 };
    max(
        300,
        (scaling_factor * (locations_len as f64).powf(1.33)) as usize,
    )
}

/// Perform a single tabu search iteration
fn perform_iteration(
    iteration: usize,
    state: &mut SearchState,
    problem_instance: &ProblemInstance,
    max_no_improvement: usize,
    tl_upper_bound_len: usize,
    tl_lower_bound_len: usize,
    aspiration_threshold: f64,
) {
    let iter_span = span!(Level::DEBUG, "iteration", iter = iteration);
    let _iter_guard = iter_span.enter();

    debug!("=== Iteration {} ===", iteration);

    state.saved_solutions.push(state.current_solution.clone());

    // Find neighbors and choose best candidate
    let swap_candidates_ind: Vec<(f64, (usize, usize))> = {
        let span = span!(Level::DEBUG, "find_neighbours");
        let _g = span.enter();
        find_neighbours(&state.current_solution, problem_instance)
    };

    let chosen_solution = choose_best_candidate(
        &swap_candidates_ind,
        &state.tabu_list,
        &state.best_so_far,
        aspiration_threshold,
        &state.parent_swap,
    );

    debug!(
        "chosen swap: {:.2}, {:?}",
        chosen_solution.0, chosen_solution.1
    );

    // Apply swap and update tabu list
    let mut final_neighbour = Route {
        route: state.current_solution.route.clone(),
        fitness: chosen_solution.0,
    };
    final_neighbour
        .route
        .swap(chosen_solution.1 .0, chosen_solution.1 .1);

    insert_and_adjust_tabu_list(&mut state.tabu_list, chosen_solution.1, state.len_tabu_list);

    // Check if improvement
    if final_neighbour.fitness < state.best_so_far.fitness {
        state.best_so_far = final_neighbour.clone();
        state.best_so_far_iteration = iteration;
        state
            .best_so_far_updates
            .push((iteration, final_neighbour.fitness));
        info!(
            "New best at iteration {}: fitness = {:.2}",
            iteration, state.best_so_far.fitness
        );
    }

    state.parent_swap = chosen_solution.1;

    // Apply diversifications and mutations
    let mut next_solution = final_neighbour;
    apply_diversifications(
        iteration,
        state,
        &mut next_solution,
        tl_upper_bound_len,
        tl_lower_bound_len,
    );

    // Evaluate new solution
    next_solution.fitness = find_fitness(
        &next_solution,
        &problem_instance.penalty_value,
        &problem_instance.num_of_trucks,
        &problem_instance.vehicle_capacities,
        &problem_instance.distance_matrix,
    );
    let next_dist = find_distance(&next_solution, &problem_instance.distance_matrix);

    // Repair if infeasible
    if next_solution.fitness > next_dist {
        info!("DEFECT - Repairing infeasible solution");
        print_solution(&next_solution, problem_instance);
        next_solution = anls_destroy_and_recreate(&mut next_solution, problem_instance);
    }

    // Update best if improved
    if next_solution.fitness < state.best_so_far.fitness {
        state.best_so_far = next_solution.clone();
        state.best_so_far_iteration = iteration;
        state
            .best_so_far_updates
            .push((iteration, next_solution.fitness));
        info!(
            "New best at iteration {}: fitness = {:.2}",
            iteration, state.best_so_far.fitness
        );
    }

    // Handle stagnation and early termination
    if state.best_so_far_iteration != iteration {
        state.stagnation += 1;
        if state.stagnation >= max_no_improvement && !state.has_ended {
            info!("ENDED EARLY AT ITERATION: {}", iteration);
            state.ended_early_value = state.best_so_far.fitness;
            state.has_ended = true;
            state.ended_early_iteration = iteration;
        } else if state.stagnation >= max_no_improvement / 2 && !state.has_ended {
            state.temperature_factor = 2;
        }
    } else {
        state.max_stagnation = max(state.stagnation, state.max_stagnation);
        state.stagnation = 0;
        state.temperature_factor = 1;
    }

    state.current_solution = next_solution;

    trace!("Current solution at end of iteration:");
    print_solution(&state.current_solution, problem_instance);
}

/// Apply diversification strategies (rollback, steer, tabu length adjustment, final mutation)
fn apply_diversifications(
    iteration: usize,
    state: &mut SearchState,
    next_solution: &mut Route,
    tl_upper_bound_len: usize,
    tl_lower_bound_len: usize,
) {
    let temp = temperature(RUNS, iteration, state.temperature_factor);

    let mutate_to_best_check = iteration % 50;
    let mutate_steer_best_check = iteration % 40;
    let mutate_tabu_len_check = iteration % 20;

    // Rollback diversification
    if state.no_seed_rng.gen::<f64>() * state.no_seed_rng.gen_range(0.3..0.6)
        <= temp * state.no_seed_rng.gen_range(0.9..1.0)
        && mutate_to_best_check == 0
        && state.saved_solutions.len() > (state.len_tabu_list * 4)
    {
        state.c1 += 1;
        *next_solution = perform_rollback(
            &state.saved_solutions,
            state.len_tabu_list,
            next_solution,
            &state.best_so_far,
        );
    } else if mutate_steer_best_check == 0 {
        // Steer towards best
        state.c2 += 1;
        let num_to_change =
            ((next_solution.route.len() as f64) * temp * state.no_seed_rng.gen::<f64>()).ceil()
                as usize;
        steer_towards_best(next_solution, &state.best_so_far, num_to_change);
    }

    // Tabu list length adjustment
    if mutate_tabu_len_check == 0 && tl_lower_bound_len < tl_upper_bound_len {
        state.c3 += 1;
        state.len_tabu_list = state
            .no_seed_rng
            .gen_range(tl_lower_bound_len..tl_upper_bound_len);
    }

    // Final mutation
    if state.no_seed_rng.gen::<f64>() * state.no_seed_rng.gen_range(0.4..0.6)
        <= temp * state.no_seed_rng.gen_range(0.8..1.0)
    {
        final_mutation(&mut state.current_solution, &mut state.rng);
        state.c4 += 1;
    }
}

/// Report final statistics and results
fn report_final_stats(state: &SearchState) {
    info!(
        "Optimization complete. Best solution found at iteration {}",
        state.best_so_far_iteration
    );

    info!("Max Stagnation: {}", state.max_stagnation);
    info!("Early end triggered: {}", state.has_ended);
    if state.has_ended {
        info!(
            "Ended early at iteration {} with fitness {:.2}",
            state.ended_early_iteration, state.ended_early_value
        );
        info!(
            "Improvement after early trigger: {:.2} ({:.2}%)",
            state.ended_early_value - state.best_so_far.fitness,
            ((state.ended_early_value - state.best_so_far.fitness) / state.ended_early_value)
                * 100.0
        );
    }

    info!(
        "Mutation counts - rollback: {}, steer: {}, tabu_len: {}, final: {}",
        state.c1, state.c2, state.c3, state.c4
    );
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    init_tracing_and_env()?;
    let db_pool = db_connection().await?;

    info!(
        "Starting VRP solver with {} locations and {} iterations",
        LOCATION_COUNT, RUNS
    );

    let google_api_key = load_google_api_key()?;

    let (locations, loc_cap, vehicle_cap) = generate_random_inputs(LOCATION_COUNT, WAREHOUSE);

    let num_of_trucks: usize = vehicle_cap.len();
    let (vehicle_cap, location_capacities) = process_inputs(vehicle_cap, loc_cap, num_of_trucks);

    let (problem_instance, initial_solution) = {
        let span = span!(Level::INFO, "setup");
        let _guard = span.enter();
        setup(
            num_of_trucks,
            &mut vehicle_cap.clone(),
            &locations,
            &mut location_capacities.clone(),
            PENALTY_VALUE as u64,
            DISTANCE_PROVIDER,
            google_api_key.as_deref(),
            db_pool,
        )
        .await
    };

    // Initialize search state
    let max_no_improvement = calculate_max_no_improvement(locations.len());
    let mut state = SearchState::new(initial_solution.clone());

    // Tabu list parameters
    let tl_upper_bound_len = 29;
    let tl_lower_bound_len = 11;
    let aspiration_threshold = 20.0;

    info!("INITIAL SOLUTION:");
    print_solution(&initial_solution, &problem_instance);

    let loop_span = span!(Level::INFO, "main_search_loop", total_iterations = RUNS);
    let _loop_guard = loop_span.enter();

    for iteration in 1..=RUNS {
        if state.has_ended {
            break;
        }

        perform_iteration(
            iteration,
            &mut state,
            &problem_instance,
            max_no_improvement,
            tl_upper_bound_len,
            tl_lower_bound_len,
            aspiration_threshold,
        );
    }

    print_solution(&state.best_so_far, &problem_instance);
    report_final_stats(&state);

    save_to_csv(
        &state.best_so_far_updates,
        state.ended_early_iteration,
        "best_so_far.csv",
    )?;

    Ok(())
}

fn save_to_csv(
    best_so_far_updates: &[(usize, f64)],
    ended_early_iteration: usize,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path(filename)?;

    wtr.write_record(["iteration", "new_best_so_far", "ended_early_iteration"])?;

    for (iteration, value) in best_so_far_updates {
        wtr.write_record([
            iteration.to_string(),
            value.to_string(),
            ended_early_iteration.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
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
        warn!(
            "Distance: {:.2}, Fitness: {:.2}, Penalty: {:.2}",
            dist, fitness, pen
        );
    } else {
        info!(
            "Distance: {:.2}, Fitness: {:.2}, Penalty: {:.2}",
            dist, fitness, pen
        );
    }

    print_location_array(solution);
    for (route, load, capacity) in partition {
        debug!("{} / {} : {:?}", load, capacity, route)
    }
}

fn partition_solution(solution: &Route, vehicle_capacity: &[u64]) -> Vec<(Vec<usize>, u64, u64)> {
    let mut route_partition: Vec<(Vec<usize>, u64)> = vec![];
    let mut temp_partition: Vec<usize> = vec![];
    let mut temp_load = 0;
    for loc in &solution.route {
        if !loc.is_warehouse {
            temp_partition.push(loc.index);
            temp_load += loc.demand
        } else {
            route_partition.push((temp_partition, temp_load));
            temp_partition = vec![];
            temp_load = 0;
        }
    }
    route_partition.push((temp_partition, temp_load));

    route_partition.sort_by_key(|&(_, value)| std::cmp::Reverse(value));
    let mut partition: Vec<(Vec<usize>, u64, u64)> = vec![];

    for (ind, (r, load)) in route_partition.iter().enumerate() {
        partition.push((r.clone(), *load, vehicle_capacity[ind]));
    }
    partition
}

fn print_location_array(solution: &Route) {
    let loc_indices: Vec<usize> = solution.route.iter().map(|loc| loc.index).collect();
    debug!("Solution route: {:?}", loc_indices)
}
