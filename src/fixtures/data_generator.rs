use crate::config::constant::{CUSTOMER_CSV_PATH, SEED, TRUCK_CAPACITIES};
use crate::setup::init::get_all_mrt_postals;
use csv::ReaderBuilder;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;
use tracing::{info, warn};

/// Reads customer postal codes from a CSV file.
/// Accepts files with or without a header and keeps at most `max_count` entries.
fn read_customer_postals_from_csv(
    csv_path: &str,
    max_count: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .trim(csv::Trim::All)
        .from_path(csv_path)?;

    let mut postals = Vec::new();
    for (idx, row) in reader.records().enumerate() {
        let record = row?;
        if let Some(raw) = record.get(0) {
            let value = raw.trim();
            if value.is_empty() {
                continue;
            }

            // Treat the first non-numeric row as a header and skip it.
            if idx == 0 && !value.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            postals.push(value.to_string());
            if postals.len() >= max_count {
                break;
            }
        }
    }

    Ok(postals)
}

/// Generates a list of random unique locations excluding the warehouse
fn random_location_generator(list_size: usize, warehouse: &str) -> Vec<String> {
    let all_postal = get_all_mrt_postals();
    let seed: u64 = SEED as u64;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut customers: Vec<String> = Vec::with_capacity(list_size);

    while customers.len() < list_size {
        let rand_index = rng.gen_range(0..all_postal.len());
        let new_loc = format!("{}", all_postal[rand_index].to_string());

        if new_loc != warehouse && !customers.contains(&new_loc) {
            customers.push(new_loc);
        }
    }

    customers.sort();

    let mut locations = Vec::with_capacity(customers.len() + 1);
    locations.push(warehouse.to_string());
    locations.extend(customers);

    locations
}

/// Loads locations from CSV with random fallback for missing entries.
fn load_locations(no_of_locations: usize, warehouse: &str) -> Vec<String> {
    let mut customers = match read_customer_postals_from_csv(CUSTOMER_CSV_PATH, no_of_locations) {
        Ok(list) => {
            info!("Loaded {} customer postal codes from CSV", list.len());
            list
        }
        Err(err) => {
            warn!(
                "Failed to read customer CSV at {}: {}. Falling back to random generation.",
                CUSTOMER_CSV_PATH, err
            );
            Vec::new()
        }
    };

    // Remove duplicates and warehouse entries while preserving order
    let mut seen = HashSet::new();
    customers.retain(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() || trimmed == warehouse {
            return false;
        }
        seen.insert(trimmed.to_string())
    });

    if customers.len() < no_of_locations {
        warn!(
            "CSV had fewer locations than requested ({} < {}), falling back to random",
            customers.len(),
            no_of_locations
        );

        // Use deterministic random generation to fill the gap
        let mut random_locations = random_location_generator(no_of_locations, warehouse);
        random_locations.remove(0); // drop warehouse

        for loc in random_locations {
            if customers.len() >= no_of_locations {
                break;
            }
            if !customers.contains(&loc) {
                customers.push(loc);
            }
        }
    }

    customers.truncate(no_of_locations);
    customers.sort();

    let mut locations = Vec::with_capacity(customers.len() + 1);
    locations.push(warehouse.to_string());
    locations.extend(customers);

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
    let locations = load_locations(no_of_locations, warehouse);
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
