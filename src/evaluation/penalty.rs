use crate::domain::types::Route;

pub fn penalty(
    solution: &Route,
    penalty_value: &u64,
    num_of_trucks: &usize,
    vehicle_cap: &[u64],
) -> f64 {
    let trucks_sorted_load = find_sorted_capacities(solution, num_of_trucks);

    let mut total_overweight = 0;
    let min_len = std::cmp::min(trucks_sorted_load.len(), vehicle_cap.len());
    for ind in 0..min_len {
        let diff: i64 = (trucks_sorted_load[ind] as i64) - (vehicle_cap[ind] as i64);
        total_overweight += std::cmp::max(0, diff);
    }

    (total_overweight as f64) * (*penalty_value as f64)
}

pub fn find_sorted_capacities(solution: &Route, num_of_trucks: &usize) -> Vec<u64> {
    let mut trucks_capacities = Vec::new();
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

    if curr_sum > 0 {
        trucks_capacities.push(curr_sum);
    }

    trucks_capacities.sort_unstable_by(|a, b| b.cmp(a));
    trucks_capacities
}
