const RUNS: usize= 20000;   
const LOCATION_COUNT: usize = 76;

// Module declarations
mod core_logic;
mod evaluation;
mod setup;
mod utils;
mod test;
mod api;


// Import functions from modules
use core_logic::{
    choose_best_candidate, final_mutation, find_neighbours, insert_and_adjust_tabu_list, perform_rollback, Location, ProblemInstance, Route
};
use evaluation::{find_distance, find_fitness, penalty, trucks_by_excess, Truck};
use rand_chacha::ChaCha8Rng;
use setup::{print_dist_matrix, setup};
use test::get_random_inputs;
use utils::{steer_towards_best, temperature};

// External crate imports
use rand::{thread_rng, Rng, SeedableRng};
use std::{cmp::max, collections::VecDeque, error::Error};
use colored::*;
use std::collections::BinaryHeap;
use csv::Writer;

// const API_KEY: &str = "AIzaSyCb_vtxCtFEnVhucj_Q7aJiL8fZhcze7jo";     // OLD
#[allow(dead_code)]
const API_KEY: &str = "AIzaSyCnwKpmjbGSNixdIo8xzbkXNR2Y_MPeGoM";
const PENALTY_VALUE: u64 = 20;

// Main function
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    // INPUT

    // DEBUGGIN FROM TEST
    let (locations,mut loc_cap, mut vehicle_cap) = get_random_inputs(LOCATION_COUNT, "207224");
    // DEBUGGIN FROM TEST

    // Input adjustment
    let num_of_trucks: usize = vehicle_cap.len();
    vehicle_cap.sort_unstable_by(|a, b| b.cmp(a));
    if num_of_trucks > 1{
        loc_cap.splice(0..0, std::iter::repeat(0).take(num_of_trucks - 2));
    }

    // SETUP
    // let mut rng = thread_rng();
    // let seed: u64 = 12345; // Set a fixed seed
    // let rng = ChaCha8Rng::seed_from_u64(seed);
    let mut no_seed_rng = thread_rng();
    let (problem_instance, initial_solution) = setup(
        num_of_trucks,
        &mut vehicle_cap,
        &locations,
        &mut loc_cap,
        PENALTY_VALUE,
        "osrm",
        None,
        // "google",
        // Some(API_KEY)
    ).await;

    // DEBUGGING

    // let initial_solution = [7, 3, 0, 9, 11, 2, 5, 6, 8, 1, 4, 10];
    // DEBUGGING

    // === SEARCH STATE ===
    let mut current_solution = initial_solution.clone();        // the solution we're currently exploring
    let mut best_so_far: Route = initial_solution.clone();      // global best solution found across all iterations
    let mut best_so_far_iteration = 0;                          // iteration index when best_so_far was last updated

    // Rolling history of solutions (can be used for rollback/trend checks).
    // Starts empty; you push into this each iter as needed.
    let mut saved_solutions: Vec<Route> = vec![];

    // Aspiration window for allowing tabu moves that are "close enough" to best.
    // NOTE: Magic number; consider tuning/deriving relative to problem scale.
    let aspiration_threshold = 20.0;

    // The swap picked in the previous iteration (by positions in the route).
    // Initialized to a sentinel "out of range" pair to avoid accidental overlap on iter 0.
    let mut parent_swap: (usize, usize) = (current_solution.route.len(), current_solution.route.len());

    // === STAGNATION TRACKING ===
    // Counts consecutive iterations with no improvement of best_so_far.
    let mut stagnation = 0;
    // Maximum observed stagnation streak (for reporting/diagnostics).
    let mut max_stagnation = 0;

    // Heuristic cap on how long we tolerate stagnation before early-stop logic kicks in.
    // Scales superlinearly with instance size; min of 300 as a floor.
    // If n < 50, be more patient (15.0), else 9.0.
    let scaling_factor = if locations.len() < 50 { 15.0 } else { 9.0 };
    let max_no_improvement = max(
        300,
        (scaling_factor * (locations.len() as f64).powf(1.33)) as usize,
    );

    // Temperature multiplier for any SA-like acceptance logic you may apply elsewhere.
    // 1 = normal; 2 = temporarily hotter when stagnating.
    let mut temperature_factor = 1;

    // === EARLY-TERMINATION SNAPSHOT ===
    // If we decide to end early, capture the state at that moment for reporting.
    let mut ended_early_value = 0.0;        // best fitness at early stop time
    let mut has_ended = false;              // whether we already flagged early end
    let mut ended_early_max_stagnation = 0; // stagnation streak at early stop
    let mut ended_early_iteration = 0;      // iteration index of early stop

    // RNG (seeded for reproducibility across runs)
    let mut rng = ChaCha8Rng::seed_from_u64(12345);

    // === TABU (current implementation uses a route history deque) ===
    // Target tabu size (will be adjusted within [tl_lower_bound_len, tl_upper_bound_len]).
    let mut len_tabu_list = 20;
    let tl_upper_bound_len = 29;
    let tl_lower_bound_len = 11;

    // Deque holding recent solutions as "tabu" states.
    // NOTE: This is state-based tabu by route/fitness; consider move- or city-pair-based tabu for robustness.
    // let mut tabu_list: VecDeque<Route> = VecDeque::new();
    let mut tabu_list: VecDeque<(usize, usize)> = VecDeque::new();



    // ======================================== DEBUGGING ========================================

    print_dist_matrix(&problem_instance.distance_matrix);
    println!("\n\nlocations: {:?}", locations);
    println!("vehicle_cap: {:?}", vehicle_cap);
    println!(
        "location Capcity: {:?}\n\n",
        problem_instance.location_demands
    );


    println!("INITIAL SOLUTION:");
    print_solution(&initial_solution, &problem_instance);
    println!();
    

    let mut c1 = 0;
    let mut c2 = 0;
    let mut c3 = 0;
    let mut c4 = 0;

    let mut best_so_far_updates: Vec<(usize, f64)> = vec![];
    // ======================================== DEBUGGING ========================================

    // MAIN LOOP
    for iteration in 1..=RUNS {
        println!(
            "============================== Iteration {} ==============================\n",
            iteration
        );

        saved_solutions.push(current_solution.clone());
        // println!();

        // +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
        // =================================== PHASE 1 ===================================
        // +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

        // O(n^3)
        // Find all the fitness' of the neighbours of current solution
        let swap_candidates_ind: Vec<(f64, (usize, usize))> =
            find_neighbours(&current_solution, &problem_instance);

        // +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
        // =================================== PHASE 2 ===================================
        // +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

        let chosen_solution = choose_best_candidate(
            &swap_candidates_ind,
            &tabu_list,
            &best_so_far,
            aspiration_threshold,
            &parent_swap,
        );

        // ======================================== DEBUGGING ========================================
        let (f, p) = chosen_solution;
        println!("chosen swap: {:.2}, {:?}\n", f, p);
        // ======================================== DEBUGGING ========================================

        let mut final_neighbour = Route {
            route: current_solution.route.clone(), // Copy the route
            fitness: chosen_solution.0,
        };
    
        final_neighbour
            .route
            .swap(chosen_solution.1 .0, chosen_solution.1 .1); // Swap in place
        

        // insert_and_adjust_tabu_list(&mut tabu_list, final_neighbour.clone(), len_tabu_list);
        insert_and_adjust_tabu_list(&mut tabu_list, (chosen_solution.1 .0, chosen_solution.1 .1), len_tabu_list);

        if final_neighbour.fitness < best_so_far.fitness {
            best_so_far = final_neighbour.clone();
            best_so_far_iteration = iteration;
            best_so_far_updates.push((iteration, final_neighbour.fitness))
        }

        // // ======================================== DEBUGGING ========================================
        print!("\nBEST SO FAR at itration {}:\n", best_so_far_iteration);
        print_solution(&best_so_far, &problem_instance);
        // println!("");

        // // ======================================== DEBUGGING ========================================

        // dont need to clone since we dont use it anymore
        parent_swap = chosen_solution.1;

        // +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
        // =================================== PHASE 3 ===================================
        // +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

        // Introduces random variation to escape local optima (similar to genetic mutation).
        // These mod values periodically trigger changes to influence the search process.
        let mutate_to_best_check = iteration % 50; // revert back to best so far if not making any progress
        let mutate_steer_best_check = iteration % 40; // change some of the current values to the best_so_far values
        let mutate_tabu_len_check = iteration % 20; // mutate the required length of the tabu list
        let temp = temperature(RUNS, iteration, temperature_factor);

        let mut next_solution = final_neighbour;

        // Perform rollback to best_so_far with some probability, influenced by Simulated Annealing
        // if rng.gen::<f64>() <= temp * rng.gen_range(0.7..1.0)
        // if no_seed_rng.gen::<f64>() <= temp * no_seed_rng.gen_range(0.9..1.0)
        if no_seed_rng.gen::<f64>() * no_seed_rng.gen_range(0.3..0.6) <= temp * no_seed_rng.gen_range(0.9..1.0)
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
            // Change SOME values in the current solution to match the best solution so far, based off some propabilty of occurence
            c2 += 1;
            let num_to_change =
                ((next_solution.route.len() as f64) * temp * no_seed_rng.gen::<f64>()).ceil() as usize;
            steer_towards_best(&mut next_solution, &best_so_far, num_to_change)
        }

        // If condition hit, then mutate the length of the tabu list
        if mutate_tabu_len_check == 0 && tl_lower_bound_len < tl_upper_bound_len {
            c3 += 1;
            len_tabu_list = no_seed_rng.gen_range(tl_lower_bound_len..tl_upper_bound_len)
        }

        // Final Mutation
        // less likely to occue later in the loop, second random f64 is to introduce some decay factor to reduce predictability
        // if no_seed_rng.gen::<f64>() <= temp * no_seed_rng.gen_range(0.8..1.0) {
        if no_seed_rng.gen::<f64>() * no_seed_rng.gen_range(0.4..0.6) <= temp * no_seed_rng.gen_range(0.8..1.0) {
            final_mutation(&mut current_solution, &mut rng);
            c4 += 1;
        }
        

        // if the new solution is infeasible or has a penalty attached to it, then we can readjust it to a better 
        // solution, such that we wouldnt have to waste iterations fixing the bad solutions
        next_solution.fitness = find_fitness(&next_solution, &problem_instance.penalty_value, &num_of_trucks, &problem_instance.vehicle_capacities, &problem_instance.distance_matrix);
        let next_dist = find_distance(&next_solution, &problem_instance.distance_matrix);
        println!("\nnext solution fitness: {}", next_solution.fitness);
        println!("next solution distance: {}", next_dist);
        if next_solution.fitness > next_dist {   // If theres a penalty
            println!("DEFECT\nSolution with defects, before ansl Solution:");
            print_solution(&next_solution, &problem_instance);
            next_solution = anls_destroy_and_recreate(&mut next_solution, &problem_instance)
        }


        // update best so far if the next solution is the best one yet.
        if next_solution.fitness < best_so_far.fitness {
            best_so_far = next_solution.clone();
            best_so_far_iteration = iteration;

            // Debugging
            best_so_far_updates.push((iteration, next_solution.fitness))
            // Debugging
        }

        // If the best solution was NOT improved in this iteration...
        if best_so_far_iteration != iteration {
            stagnation += 1; // increase consecutive "no improvement" counter

            // If we've stagnated long enough and haven't already flagged an early end...
            if stagnation >= max_no_improvement && !has_ended {
                println!(" ENDED EARLY AT ITERATION : {}", iteration);

                // Snapshoot metrics at early end for later reporting
                ended_early_value = best_so_far.fitness;
                has_ended = true;
                ended_early_max_stagnation = stagnation;
                ended_early_iteration = iteration;

                // NOTE: I currently DO NOT break here (the break is commented out),
                // so the loop continues, but I record that an early end condition occurred.
                // This is so I can see how much more I would have gasned if I didnt stop
            }
            // Softer response to mid-stagnation: temporarily increase "temperature"
            // so any SA-style acceptance or diversification becomes more permissive.
            else if stagnation >= max_no_improvement / 2 && !has_ended {
                temperature_factor = 2;
            }
        } else {
            // We DID improve best_so_far this iteration.
            // Record the maximum stagnation streak seen so far for stats.
            max_stagnation = max(stagnation, max_stagnation);

            // Reset stagnation counters and restore normal temperature.
            stagnation = 0;
            temperature_factor = 1;
        }



        current_solution = next_solution;


        println!("\nChosen NEXT Solution:");
        print_solution(&current_solution, &problem_instance);
    }


    println!("============================== END OF CALCULATION ==============================");


    // === FINAL REPORTING ===

    // Print the best solution (and the iteration when it was achieved).
    println!("\n\nFINAL ANSWER from itration {}:", best_so_far_iteration);
    print_solution(&best_so_far, &problem_instance);

    // Stagnation diagnostics
    println!("\nMax Stagnation: {}", max_stagnation);

    // Early end diagnostics
    println!("ended early value: {:.2}", ended_early_value);
    println!("end early when stagnation is: {}", max_no_improvement);
    println!("ended early stagnation: {}", ended_early_max_stagnation);
    println!("ended early iteration: {}", ended_early_iteration);

    // How many iterations were left when early-end condition triggered
    println!("ended early itr diff: {}", RUNS - ended_early_iteration);

    // Fitness delta between early-end snapshot and final best (can be negative if improved later)
    println!("ended early finess diff: {:.2}\n", ended_early_value - best_so_far.fitness);

    // Relative improvement percentage from early-end snapshot to final best
    println!(
        "fittness diff % : {:.2}%",
        ((ended_early_value - best_so_far.fitness) / ended_early_value) * 100.0
    );

    // Relative remaining-iteration percentage when early-end triggered
    let runsf64: f64 = RUNS as f64;
    let ended_early_iterationf64: f64 = ended_early_iteration as f64;
    println!(
        "itr diff % : {}\n",
        ((runsf64 - ended_early_iterationf64) / runsf64) * 100.0
    );

    // how many times each mutation was applied
    println!();
    println!("c1, c2, c3, c4: {} , {} , {} , {}", c1, c2, c3, c4);


    save_to_csv(&best_so_far_updates, ended_early_iteration, "best_so_far.csv")?;
    Ok(())

}



fn save_to_csv(best_so_far_updates: &Vec<(usize, f64)>, ended_early_iteration: usize, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path(filename)?;

    // Write header
    wtr.write_record(&["iteration", "new_best_so_far", "ended_early_iteration"])?;

    // Write data
    for (iteration, value) in best_so_far_updates {
        // let marker = if *iteration == ended_early_iteration { "1" } else { "0" };
        wtr.write_record(&[iteration.to_string(), value.to_string(), ended_early_iteration.to_string()])?;
    }

    wtr.flush()?; // Ensure data is written
    Ok(())
}



fn anls_destroy_and_recreate(solution: &mut Route, pi: &ProblemInstance, ) -> Route {
    // let mut index_dict: Vec<usize> = vec![0; solution.route.len()];
    // for ind in 0..solution.route.len(){
    //     let loc_index = solution.route[ind].index;
    //     index_dict[loc_index] = ind;
    // }

    let mut trucks = trucks_by_excess(solution, pi);
    let mut destroyed_locations_max_heap = BinaryHeap::new();
    for truck in &mut trucks{ 
        if truck.excess <= 0 {
            break
        }

        while truck.excess > 0{
            let destroyed_location = truck.route.pop().expect("Error: Tried to pop from an empty route!");
            truck.load -= destroyed_location.demand;
            truck.excess -= destroyed_location.demand as i64;
            destroyed_locations_max_heap.push(destroyed_location);
        }
    }

    // re-create the route from the destroyed locations
    for truck in trucks.iter_mut().rev() {
        if destroyed_locations_max_heap.is_empty() || truck.excess > 0 { break }

        // reinsert them in order of highest demand first into trucks, starting with the one with most space left
        while truck.excess < 0 &&                                                               // while there is still space left
                !destroyed_locations_max_heap.is_empty() &&
                truck.excess + destroyed_locations_max_heap.peek().unwrap().demand as i64 <= 0         // and addition of the new loc wont send it over
        {
            truck.route.push(destroyed_locations_max_heap.pop().unwrap())

        }
    }

    if !destroyed_locations_max_heap.is_empty(){
        if let Some(lowest_excess_truck) = trucks.iter_mut().min_by_key(|t| t.excess) {
            // Move all elements from destroyed_locations into lowest_excess_truck.route
            lowest_excess_truck.route.extend(destroyed_locations_max_heap.drain());
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
        fitness: 0.0
    };

    recreated_solution.fitness = find_fitness(&recreated_solution, &pi.penalty_value, &pi.num_of_trucks, &pi.vehicle_capacities, &pi.distance_matrix);
    recreated_solution
}

fn print_solution(solution: &Route, problem_instance: &ProblemInstance){
    let partition = partition_solution(solution, &problem_instance.vehicle_capacities);

    let dist = find_distance(solution, &problem_instance.distance_matrix);
    let fitness = find_fitness( solution, 
        &problem_instance.penalty_value, &problem_instance.num_of_trucks,
        &problem_instance.vehicle_capacities, &problem_instance.distance_matrix);
    let pen = penalty(solution, &problem_instance.penalty_value, &problem_instance.num_of_trucks, &problem_instance.vehicle_capacities);

    if pen > 0.0 {
        // println!("{}", format!("Distance: {:.2}, {}", dist, format!("Fitness: {:.2}, Penalty: {:.2}", fitness, pen).red()));
        println!("Distance: {:.2}, {}", dist,
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
    for (route, load, capacity) in partition{
        println!("{} / {} : {:?}", load, capacity, route)
    }

}

fn partition_solution(solution: &Route, vehicle_capacity: &[u64]) -> Vec<(Vec<usize>, u64, u64)>{
    let mut route_partition: Vec<(Vec<usize>, u64)> = vec![];
    let mut temp_partition: Vec<usize> = vec![];
    let mut temp_load = 0;
    for loc in &solution.route{
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
        partition.push((r.clone(), *load, vehicle_capacity[ind]));      // Clone to avoid ownership issues
    } 
    partition
}

fn print_location_array(solution: &Route){
    let mut loc_indices: Vec<usize> = vec![]; 
    for loc in &solution.route{
        loc_indices.push(loc.index);
    }
    println!("solution route: {:?}", loc_indices)
}
