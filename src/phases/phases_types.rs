pub struct ProblemInstance {
    #[allow(dead_code)]
    pub locations_string: Vec<String>,
    pub distance_matrix: Vec<Vec<f64>>,
    pub vehicle_capacities: Vec<u64>,
    pub location_demands: Vec<u64>,
    pub num_of_trucks: usize,
    pub penalty_value: u64,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub route: Vec<Location>,
    pub fitness: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub index: usize,
    pub demand: u64,
    pub is_warehouse: bool,
}

// Implement `Ord` and `PartialOrd` to sort by demand in descending order (max heap)
impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.demand.cmp(&self.demand) // Reverse order for max heap
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

