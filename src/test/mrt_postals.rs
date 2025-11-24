use std::fs;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;

/// Struct to match the JSON structure
#[derive(Debug, Deserialize)]
struct MRTLocation {
    #[serde(rename = "Possible Locations")]
    possible_locations: Vec<LocationData>,
}

#[derive(Debug, Deserialize)]
struct LocationData {
    #[serde(rename = "POSTAL")]
    postal: String,
}

/// Reads the JSON file and returns a list of all MRT postal codes
fn get_all_mrt_postals() -> Vec<String> {
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

/// Generates a list of random unique locations excluding the warehouse
fn random_location_generator(list_size: usize, warehouse: &str) -> Vec<String> {
    let all_postal = get_all_mrt_postals();
    // let mut rng = rand::thread_rng();
    let seed: u64 = 12345; // Set a fixed seed
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // let mut locations = vec![format!("{} Singapore", warehouse)];
    let mut locations = vec![format!("{}", warehouse)];

    while locations.len() < list_size + 1 {
        // +1 to include warehouse
        let rand_index = rng.gen_range(0..all_postal.len());
        // let new_loc = format!("{} Singapore", all_postal[rand_index]);
        let new_loc = format!("{}", all_postal[rand_index]);

        if !locations.contains(&new_loc) {
            locations.push(new_loc);
        }
    }

    locations
}

/// Generates random capacities for each location (excluding warehouse)
fn random_capacity_generator(locations: &[String]) -> Vec<u64> {
    // let mut rng = rand::thread_rng();
    let seed: u64 = 12345; // Set a fixed seed
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut capacity = vec![0]; // Warehouse always has zero capacity

    for _ in 1..locations.len() {
        let random_cap = rng.gen_range(100_000..=150_000);
        capacity.push(random_cap);
    }

    capacity
}

/// Generates vehicle capacities based on total demand
fn vehicle_cap_generator(capacity: &[u64], base1_vcap: &[u64]) -> (Vec<u64>, usize) {
    let base_sum: u64 = base1_vcap.iter().sum(); // Total base vehicle capacity
    let cap_sum: u64 = capacity.iter().sum(); // Total demand
    let factor = (cap_sum / base_sum) + 1; // How many times we need to repeat the base capacities

    // Repeat base capacities until we meet the total required capacity
    let vehicle_cap: Vec<u64> = base1_vcap
        .iter()
        .cloned()
        .cycle() // Repeat elements infinitely
        .take(base1_vcap.len() * factor as usize) // Take enough elements
        .collect();

    let num_vehicles = vehicle_cap.len();

    (vehicle_cap, num_vehicles)
}

pub fn get_random_inputs(
    no_of_locations: usize,
    warehouse: &str,
) -> (Vec<std::string::String>, Vec<u64>, Vec<u64>) {
    // Generate locations
    let locations = random_location_generator(no_of_locations, warehouse);
    println!("Generated Locations: {:?}", locations);

    // Generate capacities
    let location_capacities = random_capacity_generator(&locations);
    println!("Generated Capacities: {:?}", location_capacities);

    // Generate vehicle capacities
    let base_vehicle_cap = vec![1_000_000, 500_000];
    let (vehicle_cap, num_vehicles) =
        vehicle_cap_generator(&location_capacities, &base_vehicle_cap);
    println!(
        "Generated Vehicle Capacities: {:?}, Number of Vehicles: {}",
        vehicle_cap, num_vehicles
    );

    (locations, location_capacities, vehicle_cap)
}
