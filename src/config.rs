pub mod constant {
    pub(crate) const RUNS: usize = 2000;
    pub(crate) const LOCATION_COUNT: usize = 76;
    pub(crate) const SEED: usize = 64;
    pub(crate) const PENALTY_VALUE: usize = 20;
    pub(crate) const DISTANCE_PROVIDER: &str = "osrm"; // "osrm" or "google"
    pub(crate) const WAREHOUSE: &str = "207224"; // warehouse postal code
    pub(crate) const CUSTOMER_CSV_PATH: &str = "data/customers.csv"; // customer postal codes

    // Fixed fleet configuration - number of trucks is derived from array length
    pub(crate) const TRUCK_CAPACITIES: [u64; 10] = [
        1_000_000, 1_000_000, 1_000_000, 1_000_000, 1_000_000, 1_000_000, 1_000_000, 1_000_000,
        1_000_000, 1_000_000,
    ];
}
