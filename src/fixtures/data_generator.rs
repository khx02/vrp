use crate::config::constant::{CUSTOMER_CSV_PATH, TRUCK_CAPACITIES};
use csv::ReaderBuilder;
use std::collections::HashSet;
use tracing::{info, warn};

/// Reads unique customer postal codes and their demands from CSV.
///
/// Supports files with or without a header row.
/// Returns customers sorted by postal code for determinism.
fn read_customer_postals_from_csv(
    csv_path: &str,
    max_count: usize,
    warehouse: &str,
) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .trim(csv::Trim::All)
        .from_path(csv_path)?;

    let mut customers = Vec::with_capacity(max_count);
    let mut seen = HashSet::new();

    for (idx, row) in reader.records().enumerate() {
        let record = row?;
        if record.is_empty() {
            continue;
        }

        let raw_postal = match record.get(0) {
            Some(v) => v.trim(),
            None => continue,
        };

        // Treat the first non-numeric row as a header and skip it.
        if idx == 0 && !raw_postal.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        if raw_postal.is_empty() || raw_postal == warehouse {
            continue;
        }

        if !seen.insert(raw_postal.to_string()) {
            continue;
        }

        let demand = match record.get(1) {
            Some(raw_demand) if !raw_demand.trim().is_empty() => {
                raw_demand.trim().parse::<u64>().map_err(|_| {
                    format!("Invalid demand value '{}' for {}", raw_demand, raw_postal)
                })?
            }
            _ => {
                return Err(format!(
                    "Missing demand for postal {} in {}. Please regenerate the CSV with demands.",
                    raw_postal, csv_path
                )
                .into());
            }
        };

        customers.push((raw_postal.to_string(), demand));

        if customers.len() >= max_count {
            break;
        }
    }

    if customers.is_empty() {
        return Err(format!(
            "Customer CSV at {} contained no valid customer rows (excluding warehouse {}).",
            csv_path, warehouse
        )
        .into());
    }

    if customers.len() < max_count {
        warn!(
            "Customer CSV at {} provided {} unique entries, less than requested {}. Using available rows only.",
            csv_path,
            customers.len(),
            max_count
        );
    }

    customers.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(customers)
}

/// Generates fixed vehicle capacities (no longer based on total demand)
fn vehicle_cap_generator() -> (Vec<u64>, usize) {
    // Use fixed capacities from config - number of trucks derived from array length
    let vehicle_cap: Vec<u64> = TRUCK_CAPACITIES.to_vec();
    let num_vehicles = vehicle_cap.len();

    (vehicle_cap, num_vehicles)
}

/// Load locations, customer demands, and vehicle capacities from CSV (deterministic)
pub fn load_inputs_from_csv(
    no_of_locations: usize,
    warehouse: &str,
) -> Result<(Vec<String>, Vec<u64>, Vec<u64>), Box<dyn std::error::Error>> {
    let customers = read_customer_postals_from_csv(CUSTOMER_CSV_PATH, no_of_locations, warehouse)?;
    info!(
        "Loaded {} customer rows from {}",
        customers.len(),
        CUSTOMER_CSV_PATH
    );

    let take_n = customers.len().min(no_of_locations);

    let mut locations: Vec<String> = Vec::with_capacity(take_n + 1);
    let mut customer_demands: Vec<u64> = Vec::with_capacity(take_n + 1);

    locations.push(warehouse.to_string());
    customer_demands.push(0);

    for (postal, demand) in customers.into_iter().take(take_n) {
        locations.push(postal);
        customer_demands.push(demand);
    }

    let total_demand: u64 = customer_demands.iter().sum();
    info!("Customer Demands: {:?}", customer_demands);
    info!("Total Demand: {}", total_demand);

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

    Ok((locations, customer_demands, vehicle_cap))
}
