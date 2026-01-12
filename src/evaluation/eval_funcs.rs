use crate::phases::types::*;

pub fn find_fitness(
    solution: &Route,
    penalty_value: &u64,
    num_of_trucks: &usize,
    vehicle_cap: &[u64],
    dm: &[Vec<f64>],
) -> f64 {
    find_distance(solution, dm) + penalty(solution, penalty_value, num_of_trucks, vehicle_cap)
}

pub fn find_distance(solution: &Route, dm: &[Vec<f64>]) -> f64 {
    let r: &Vec<Location> = &solution.route;
    if r.is_empty() {
        return 0.0;
    }

    let warehouse_to_first_loc = dist_between(0, r[0].index, dm);
    let last_loc_to_warehouse = dist_between(r[r.len() - 1].index, 0, dm);

    let mut total_dist = 0.0;
    for i in 0..solution.route.len() - 1 {
        total_dist += dist_between(r[i].index, r[i + 1].index, dm);
    }

    warehouse_to_first_loc + total_dist + last_loc_to_warehouse
}

pub fn penalty(
    solution: &Route,
    penalty_value: &u64,
    num_of_trucks: &usize,
    vehicle_cap: &[u64],
) -> f64 {
    let trucks_sorted_load = find_sorted_capacities(solution, num_of_trucks);
    // Note that the vehicle_cap is already sorted in descending order
    // println!("trucks sorted load: {:?}", trucks_sorted_load);
    // println!("vehicle cap: {:?}", vehicle_cap);

    let mut total_overweight = 0;
    // Prevent index out-of-bounds errors
    let min_len = std::cmp::min(trucks_sorted_load.len(), vehicle_cap.len());
    for ind in 0..min_len {
        // println!("{} - {} = {}",trucks_sorted_load[ind], vehicle_cap[ind], (trucks_sorted_load[ind] as i64) - (vehicle_cap[ind] as i64));
        let diff: i64 = (trucks_sorted_load[ind] as i64) - (vehicle_cap[ind] as i64);
        total_overweight += std::cmp::max(0, diff);
    }

    // total_overweight * penalty_value
    (total_overweight as f64) * (*penalty_value as f64)
}

pub fn find_sorted_capacities(solution: &Route, num_of_trucks: &usize) -> Vec<u64> {
    let mut trucks_capacities = Vec::new(); // Initialize as mutable
    let mut curr_sum = 0;
    let r = &solution.route;

    for loc in r.iter() {
        if loc.index >= (*num_of_trucks - 1) {
            curr_sum += loc.demand
        } else if curr_sum == 0 {
            continue;
        } else {
            trucks_capacities.push(curr_sum);
            curr_sum = 0;
        }
    }

    // Ensure the last accumulated sum is added
    if curr_sum > 0 {
        trucks_capacities.push(curr_sum);
    }

    // trucks_capacities.push(curr_sum);

    // Sort in descending order
    trucks_capacities.sort_unstable_by(|a, b| b.cmp(a));

    trucks_capacities
}

pub fn dist_between(from_loc: usize, to_loc: usize, dm: &[Vec<f64>]) -> f64 {
    dm[from_loc][to_loc]
}

pub struct Truck {
    pub load: u64,
    pub capacity: u64,
    pub excess: i64,
    pub ending_warehouse: Option<usize>,
    pub route: Vec<Location>,
}

pub fn find_sorted_capacities2(solution: &Route, num_of_trucks: &usize) -> Vec<Truck> {
    let mut trucks: Vec<Truck> = vec![];
    let r = &solution.route;

    let mut temp_truck: Truck = Truck {
        load: 0,
        capacity: 0,
        excess: 0,
        ending_warehouse: None,
        route: vec![],
    };
    for loc in r.iter() {
        if loc.index >= (*num_of_trucks - 1) {
            // is not a warehouse
            temp_truck.load += loc.demand;
            temp_truck.route.push(loc.clone());
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

    // Ensure the last accumulated sum is added
    temp_truck.ending_warehouse = Some(solution.route.len());
    trucks.push(temp_truck);

    trucks.sort_by_key(|t| std::cmp::Reverse(t.load));

    trucks
}

pub fn trucks_by_excess(solution: &Route, pi: &ProblemInstance) -> Vec<Truck> {
    let mut trucks = find_sorted_capacities2(solution, &pi.num_of_trucks);
    for (ind, vc) in pi.vehicle_capacities.iter().enumerate() {
        trucks[ind].capacity = *vc;
        trucks[ind].excess = (trucks[ind].load as i64) - (trucks[ind].capacity as i64);
    }

    trucks.sort_by_key(|t| std::cmp::Reverse(t.excess));

    trucks
}
