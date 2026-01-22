use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sqlx::SqlitePool;
use std::fs;
use tracing::{debug, info};

// Internal module imports
use crate::distance::matrix::create_dm;
use crate::domain::types::{Location, MRTLocation, ProblemInstance, Route};
use crate::evaluation::fitness::find_fitness;

pub async fn setup(
    vehicle_cap: &mut [u64],
    pre_locations: &[String],
    loc_capacity: &mut Vec<u64>,
    penalty: u64,
    source: &str,
    api_key: Option<&str>,
    pool: SqlitePool,
) -> (ProblemInstance, Route) {
    let num_of_trucks = vehicle_cap.len();
    info!(
        "Starting setup with {} trucks, {} locations",
        num_of_trucks,
        pre_locations.len()
    );

    // Sort vehicle capacities in descending order
    vehicle_cap.sort_unstable_by(|a, b| b.cmp(a));

    // Expand locations and demands with one depot per truck (index range 0..num_of_trucks)
    let mut locations: Vec<String> = pre_locations.to_vec();
    let mut demands: Vec<u64> = loc_capacity.clone();

    if num_of_trucks > 1 {
        let warehouse = locations.get(0).cloned().expect("Missing warehouse entry");
        let extra_depots = num_of_trucks - 1;

        locations.splice(0..0, std::iter::repeat_n(warehouse, extra_depots));
        demands.splice(0..0, std::iter::repeat_n(0, extra_depots));
    }

    let dm = create_dm(source, locations.clone(), num_of_trucks, api_key, pool).await;

    let problem_instance = ProblemInstance {
        locations_string: locations.clone(),
        distance_matrix: dm.clone(),
        vehicle_capacities: vehicle_cap.to_vec(),
        location_demands: demands.clone(),
        num_of_trucks: vehicle_cap.len(),
        penalty_value: penalty,
    };

    // Generate and shuffle initial solution indices across expanded locations
    let mut initial_solution_indices: Vec<usize> = (0..locations.len()).collect();
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
            demand: demands[ind],
            is_warehouse: ind < num_of_trucks,
        };
        route.route.push(temp_location);
    }

    route.fitness = find_fitness(&route, &penalty, &num_of_trucks, vehicle_cap, &dm);

    info!("Setup completed successfully");

    (problem_instance, route)
}

/// Reads the JSON file and returns a list of all MRT postal codes
pub fn get_all_mrt_postals() -> Vec<String> {
    // Read JSON file (force panic if it fails)
    let file_content = fs::read_to_string("mrt_data.json").expect("Failed to read mrt_data.json");

    // Deserialize JSON into Vec<MRTLocation>
    let all_mrt_postal: Vec<MRTLocation> =
        serde_json::from_str(&file_content).expect("Failed to parse JSON");

    // Extract postal codes
    all_mrt_postal
        .iter()
        .map(|mrt| mrt.possible_locations[0].postal.clone())
        .collect()
}

// Print distance matrix for debugging
pub fn print_dist_matrix(dist_m: &Vec<Vec<f64>>) {
    debug!("Distance matrix:");
    for row in dist_m {
        debug!("{:?}", row);
    }
}
