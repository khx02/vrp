use std::cmp::Reverse;

use crate::domain::types::{ProblemInstance, Route, Truck};

/// Partition a route into per-truck segments using depot markers and sort by load (descending).
pub fn partition_trucks_sorted_by_load(solution: &Route, num_of_trucks: &usize) -> Vec<Truck> {
    let mut trucks: Vec<Truck> = vec![];
    let r = &solution.route;

    let mut temp_truck = Truck {
        load: 0,
        capacity: 0,
        excess: 0,
        ending_warehouse: None,
        route: vec![],
    };

    for loc in r.iter() {
        if loc.index >= (*num_of_trucks - 1) {
            temp_truck.load += loc.demand;
            temp_truck.route.push(*loc);
        } else {
            temp_truck.ending_warehouse = Some(loc.index);
            trucks.push(temp_truck);
            temp_truck = Truck {
                load: 0,
                capacity: 0,
                excess: 0,
                ending_warehouse: None,
                route: vec![],
            };
        }
    }

    temp_truck.ending_warehouse = Some(solution.route.len());
    trucks.push(temp_truck);

    trucks.sort_by_key(|t| Reverse(t.load));
    trucks
}

/// Partition a route, attach capacities, compute excess, and sort by excess (descending).
pub fn trucks_by_excess(solution: &Route, pi: &ProblemInstance) -> Vec<Truck> {
    let mut trucks = partition_trucks_sorted_by_load(solution, &pi.num_of_trucks);
    for (ind, vc) in pi.vehicle_capacities.iter().enumerate() {
        if let Some(truck) = trucks.get_mut(ind) {
            truck.capacity = *vc;
            truck.excess = (truck.load as i64) - (truck.capacity as i64);
        }
    }

    trucks.sort_by_key(|t| Reverse(t.excess));
    trucks
}
