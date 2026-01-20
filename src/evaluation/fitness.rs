use crate::domain::types::{Location, Route};

pub fn find_fitness(
    solution: &Route,
    penalty_value: &u64,
    num_of_trucks: &usize,
    vehicle_cap: &[u64],
    dm: &[Vec<f64>],
) -> f64 {
    find_distance(solution, dm)
        + crate::evaluation::penalty::penalty(solution, penalty_value, num_of_trucks, vehicle_cap)
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

pub fn dist_between(from_loc: usize, to_loc: usize, dm: &[Vec<f64>]) -> f64 {
    dm[from_loc][to_loc]
}
