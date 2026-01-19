use crate::config::constant::{SEED, TRUCK_SIZES};
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

/// Generates random capacities for each location (excluding warehouse)
fn random_capacity_generator(locations: &[String]) -> Vec<u64> {
    let seed: u64 = SEED as u64;
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
    let base_sum: u64 = base1_vcap.iter().sum();
    let cap_sum: u64 = capacity.iter().sum();
    let factor = (cap_sum / base_sum) + 1;

    let vehicle_cap: Vec<u64> = base1_vcap
        .iter()
        .cloned()
        .cycle()
        .take(base1_vcap.len() * factor as usize)
        .collect();

    let num_vehicles = vehicle_cap.len();

    (vehicle_cap, num_vehicles)
}

/// Generate random locations, capacities, and vehicle capacities for testing
pub fn generate_random_inputs(
    no_of_locations: usize,
    warehouse: &str,
) -> (Vec<String>, Vec<u64>, Vec<u64>) {
    let locations = random_location_generator(no_of_locations, warehouse);
    info!("Generated Locations: {:?}", locations);

    let location_capacities = random_capacity_generator(&locations);
    info!("Generated Capacities: {:?}", location_capacities);

    let base_vehicle_cap: Vec<u64> = Vec::from(TRUCK_SIZES)
        .into_iter()
        .map(|x| x as u64)
        .collect();
    let (vehicle_cap, num_vehicles) =
        vehicle_cap_generator(&location_capacities, &base_vehicle_cap);
    info!(
        "Generated Vehicle Capacities: {:?}, Number of Vehicles: {}",
        vehicle_cap, num_vehicles
    );

    (locations, location_capacities, vehicle_cap)
}
