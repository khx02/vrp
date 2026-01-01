// External crates
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sqlx::SqlitePool;

// Internal module imports
use crate::api::google_api::create_dm_google;
use crate::api::osrm_api::{convert_to_coords, create_dm_osrm};
use crate::{
    core_logic::{Location, ProblemInstance},
    evaluation::find_fitness,
    Route,
};

use tracing::{debug, error, info};

pub async fn setup(
    num_of_trucks: usize,
    vehicle_cap: &mut [u64],
    pre_locations: &[String],
    loc_capacity: &mut Vec<u64>,
    penalty: u64,
    source: &str,
    api_key: Option<&str>,
    pool: SqlitePool,
) -> (ProblemInstance, Route) {
    info!(
        "Starting setup with {} trucks, {} locations",
        num_of_trucks,
        pre_locations.len()
    );

    // Sort vehicle capacities in descending order
    vehicle_cap.sort_unstable_by(|a, b| b.cmp(a));

    let dm = create_dm(source, pre_locations.to_vec(), num_of_trucks, api_key, pool).await;

    // Insert dummy warehouse demands for multi-truck scenarios
    if num_of_trucks > 1 {
        loc_capacity.splice(0..0, std::iter::repeat(0).take(num_of_trucks - 2));
    }

    let problem_instance = ProblemInstance {
        locations_string: pre_locations.to_owned(),
        distance_matrix: dm.clone(),
        vehicle_capacities: vehicle_cap.to_vec(),
        location_demands: loc_capacity.clone(),
        num_of_trucks: vehicle_cap.len(),
        penalty_value: penalty,
    };

    // Generate and shuffle initial solution indices
    let mut initial_solution_indices: Vec<usize> = (0..pre_locations.len()).collect();
    debug!("Initial solution indices: {:?}", initial_solution_indices);

    let seed: u64 = 12345;
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
        route.route.push(temp_location);
    }

    route.fitness = find_fitness(&route, &penalty, &num_of_trucks, vehicle_cap, &dm);

    info!("Setup completed successfully");

    (problem_instance, route)
}

async fn create_dm(
    source: &str,
    locations: Vec<String>,
    num_of_trucks: usize,
    api_key: Option<&str>,
    pool: SqlitePool,
) -> Vec<Vec<f64>> {
    info!(
        "Creating distance matrix using source '{}' ({} locations, {} trucks)",
        source,
        locations.len(),
        num_of_trucks
    );

    match source {
        "google" => {
            let api_key = api_key.expect("API key required for Google source");
            debug!("Fetching distance matrix from Google Maps API");
            match create_dm_google(locations, num_of_trucks, api_key).await {
                Ok(matrix) => {
                    info!("Successfully retrieved matrix from Google API");
                    matrix
                }
                Err(e) => {
                    error!("Google API request failed: {:?}", e);
                    vec![vec![]]
                }
            }
        }

        "osrm" => {
            let mut target_locations = locations;

            // For multi-truck scenarios, repeat warehouse location
            if num_of_trucks > 1 {
                let warehouse = target_locations[0].clone();
                target_locations.splice(0..0, std::iter::repeat(warehouse).take(num_of_trucks - 2));
                debug!("Added {} repeated warehouse locations", num_of_trucks - 2);
            }

            let coords = convert_to_coords(&pool, target_locations).await;
            debug!("Converted to {} coordinates", coords.len());

            if coords.len() < 2 {
                error!("Insufficient valid coordinates for distance matrix");
                return vec![vec![]];
            }

            match create_dm_osrm(&coords).await {
                Some(matrix) => {
                    info!("Successfully retrieved matrix from OSRM");
                    matrix
                }
                None => {
                    error!("OSRM failed to return a valid distance matrix");
                    vec![vec![]]
                }
            }
        }

        _ => {
            error!("Unknown distance matrix source: {}", source);
            vec![vec![]]
        }
    }
}

// Print distance matrix for debugging
pub fn print_dist_matrix(dist_m: &Vec<Vec<f64>>) {
    debug!("Distance matrix:");
    for row in dist_m {
        debug!("{:?}", row);
    }
}

