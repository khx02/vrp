use crate::config::constant::{SEED, TRUCK_CAPACITIES};
use crate::setup::init::get_all_mrt_postals;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tracing::info;

/// Generates a list of random unique locations excluding the warehouse
fn random_location_generator(list_size: usize, warehouse: &str) -> Vec<String> {
    let all_postal = get_all_mrt_postals();
    let seed: u64 = SEED as u64;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut locations = vec![format!("{}", warehouse)];

    while locations.len() < list_size + 1 {
        let rand_index = rng.gen_range(0..all_postal.len());
        let new_loc = format!("{}", all_postal[rand_index]);

        if !locations.contains(&new_loc) {
            locations.push(new_loc);
        }
    }

    locations
}

/// Generates random customer demands for each location
///
/// Returns a vector where:
/// - First element (warehouse) is always 0
/// - Subsequent elements are random demands in range [100_000, 150_000]
fn generate_customer_demands(locations: &[String]) -> Vec<u64> {
    let seed: u64 = SEED as u64;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut demands = vec![0]; // Warehouse always has zero demand

    for _ in 1..locations.len() {
        let random_demand = rng.gen_range(100_000..=150_000);
        demands.push(random_demand);
    }

    demands
}

/// Generates fixed vehicle capacities (no longer based on total demand)
fn vehicle_cap_generator() -> (Vec<u64>, usize) {
    // Use fixed capacities from config - number of trucks derived from array length
    let vehicle_cap: Vec<u64> = TRUCK_CAPACITIES.to_vec();
    let num_vehicles = vehicle_cap.len();

    (vehicle_cap, num_vehicles)
}

/// Generate random locations, customer demands, and vehicle capacities for testing
pub fn generate_random_inputs(
    no_of_locations: usize,
    warehouse: &str,
) -> (Vec<String>, Vec<u64>, Vec<u64>) {
    let locations = random_location_generator(no_of_locations, warehouse);
    info!("Generated Locations: {:?}", locations);

    let customer_demands = generate_customer_demands(&locations);
    let total_demand: u64 = customer_demands.iter().sum();
    info!("Customer Demands: {:?}", customer_demands);
    info!("Total Demand: {}", total_demand);

    // Use fixed fleet configuration
    let (vehicle_cap, num_vehicles) = vehicle_cap_generator();
    let total_capacity: u64 = vehicle_cap.iter().sum();
    info!(
        "Fixed Fleet - Number of Trucks: {}, Total Capacity: {}",
        num_vehicles, total_capacity
    );
    info!("Vehicle Capacities: {:?}", vehicle_cap);

    if total_capacity < total_demand {
        info!(
            "WARNING: Total fleet capacity ({}) is less than total demand ({}). Solver will handle via penalties.",
            total_capacity, total_demand
        );
    }

    (locations, customer_demands, vehicle_cap)
}
