// External crates
use rand::{seq::SliceRandom, thread_rng, SeedableRng};
use rand_chacha::ChaCha8Rng; // Merged rand imports

// use crate::api;
// Internal module imports
use crate::api::google_api::create_dm_google;
use crate::api::osrm_api::convert_to_coords;
use crate::api::osrm_api::create_dm_osrm;
use crate::{
    core_logic::{Location, ProblemInstance},
    evaluation::find_fitness,
    Route,
};

// use colored::*;

pub async fn setup(
    num_of_trucks: usize,
    vehicle_cap: &mut [u64],
    pre_locations: &[String],
    loc_capacity: &mut Vec<u64>,
    penalty: u64,
    source: &str,
    api_key: Option<&str>,
) -> (ProblemInstance, Route) {
    // First sort the vehicle capacity
    vehicle_cap.sort_unstable_by(|a, b| b.cmp(a));

    let dm = create_dm(source, pre_locations.to_vec(), num_of_trucks, api_key).await;

    if num_of_trucks > 1 {
        loc_capacity.splice(0..0, std::iter::repeat(0).take(num_of_trucks - 2));
    }

    // creating the problem statement:
    let problem_instance = ProblemInstance {
        locations_string: pre_locations.to_owned(),
        distance_matrix: dm.clone(),
        vehicle_capacities: vehicle_cap.to_vec(),
        location_demands: loc_capacity.clone(),
        num_of_trucks: vehicle_cap.len(),
        penalty_value: penalty,
    };

    // Generate a list of indices from 1 to len(locations)
    let mut initial_solution_indices: Vec<usize> = (0..pre_locations.len()).collect();
    println!("{:?}", initial_solution_indices);
    // Shuffle the solution
    // let mut rng = thread_rng();
    let seed: u64 = 12345; // Set a fixed seed
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    initial_solution_indices.shuffle(&mut rng);

    let mut route = Route {
        route: vec![],
        fitness: 0.0,
    };

    for ind in initial_solution_indices {
        let temp_location = Location {
            index: ind,
            demand: loc_capacity[ind],
            is_warehouse: ind < num_of_trucks - 1,
        };
        route.route.push(temp_location)
    }

    route.fitness = find_fitness(&route, &penalty, &num_of_trucks, vehicle_cap, &dm);

    (problem_instance, route)
}

async fn create_dm(
    source: &str,
    locations: Vec<String>,
    num_of_trucks: usize,
    api_str: Option<&str>,
) -> Vec<Vec<f64>> {
    match source {
        "google" => {
            // 1. Ensure we have an API key for Google
            let api_key = match api_str {
                Some(key) => key,
                None => {
                    println!("Error: Google source requires an API key");
                    return vec![vec![]]; // Return an empty matrix
                }
            };

            // 2. Call Google DM function
            match create_dm_google(locations, num_of_trucks, api_key).await {
                Ok(matrix) => matrix, // Return the matrix on success
                Err(e) => {
                    println!("Error calling Google API: {:?}", e);
                    vec![vec![]] // Return an empty matrix on error
                }
            }
        }

        "osrm" => {
            let coords: Vec<(f64, f64)>;
            if num_of_trucks > 1 {
                let mut cloned_locations = locations.clone();
                // Insert (num_of_trucks - 2) copies of the first location at the front
                let first_location = cloned_locations[0].clone();
                cloned_locations.splice(
                    0..0,
                    std::iter::repeat(first_location).take(num_of_trucks - 2),
                );
                // Now `cloned_locations` has the repeated items at the front
                coords = convert_to_coords(cloned_locations).await;
            } else {
                coords = convert_to_coords(locations).await;
            }

            if coords.len() < 2 {
                // check if there are sufficient locations
                println!("Not enough valid coordinates to build a matrix.");
                return vec![vec![]];
            }

            // 2. Get OSRM distance matrix
            match create_dm_osrm(&coords).await {
                Some(matrix) => matrix,
                None => {
                    println!("Failed to get distance matrix from OSRM.");
                    vec![vec![]]
                }
            }
        }

        // Additional sources if needed...
        _ => {
            println!("Unknown source. Doing nothing.");
            vec![vec![]]
        }
    }
}

// =========================================== OSRM API ===========================================

// =========================================== OSRM API ===========================================

// ========================================== GOOGLE API ==========================================

// ========================================== GOOGLE API ==========================================

// DEBUGGING
// Function to print the distance matrix
pub fn print_dist_matrix(dist_m: &Vec<Vec<f64>>) {
    for row in dist_m {
        println!("{:?}", row);
    }
}
