use std::collections::BinaryHeap;

use crate::domain::solution::trucks_by_excess;
use crate::domain::types::{Location, ProblemInstance, Route, Truck};
use crate::evaluation::fitness::find_fitness;

/// ALNS-style destroy and repair: removes infeasible locations and reinserts them
pub fn alns_destroy_and_recreate(solution: &mut Route, pi: &ProblemInstance) -> Route {
    let mut trucks = trucks_by_excess(solution, pi);
    let mut destroyed_locations_max_heap = BinaryHeap::new();

    // Destroy: remove locations from overloaded trucks
    for truck in &mut trucks {
        if truck.excess <= 0 {
            break;
        }

        while truck.excess > 0 {
            let destroyed_location = truck
                .route
                .pop()
                .expect("Error: Tried to pop from an empty route!");
            truck.load -= destroyed_location.demand;
            truck.excess -= destroyed_location.demand as i64;
            destroyed_locations_max_heap.push(destroyed_location);
        }
    }

    // Repair: reinsert into underutilized trucks (prioritizing higher-demand locations)
    for truck in trucks.iter_mut().rev() {
        if destroyed_locations_max_heap.is_empty() || truck.excess > 0 {
            break;
        }

        while truck.excess < 0
            && !destroyed_locations_max_heap.is_empty()
            && truck.excess + destroyed_locations_max_heap.peek().unwrap().demand as i64 <= 0
        {
            truck
                .route
                .push(destroyed_locations_max_heap.pop().unwrap())
        }
    }

    // Fallback: insert remaining locations into truck with lowest excess
    if !destroyed_locations_max_heap.is_empty() {
        if let Some(lowest_excess_truck) = trucks.iter_mut().min_by_key(|t| t.excess) {
            lowest_excess_truck
                .route
                .extend(destroyed_locations_max_heap.drain());
        }
    }

    recreate_route_from_trucks(&mut trucks, pi)
}

/// Reconstruct a single Route from partitioned trucks
fn recreate_route_from_trucks(trucks: &mut [Truck], pi: &ProblemInstance) -> Route {
    let mut recreated_route: Vec<Location> = vec![];
    let mut partition_counter = 0;

    for (i, truck) in trucks.iter().enumerate() {
        recreated_route.extend(truck.route.clone());

        // Add warehouse markers between routes (except after last truck)
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
        fitness: 0.0,
    };

    // Recalculate fitness for the repaired solution
    recreated_solution.fitness = find_fitness(
        &recreated_solution,
        &pi.penalty_value,
        &pi.num_of_trucks,
        &pi.vehicle_capacities,
        &pi.distance_matrix,
    );

    recreated_solution
}
