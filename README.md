# VRP Solver

Vehicle Routing Problem (VRP) solver written in Rust. It builds distance matrices (OSRM/Google), generates initial solutions, and runs a tabu-search–style metaheuristic with diversification to find good routes. Outputs progress to `best_so_far.csv` and provides a Python plot script.

## Project Layout
```
├── Cargo.toml
├── src/
│   ├── bin/
│   │   └── vrp-solver.rs        # thin binary entrypoint
│   ├── lib.rs                   # library surface (re-exports)
│   ├── domain/                  # core data types
│   │   ├── types.rs             # Location, Route, Truck, ProblemInstance
│   │   └── solution.rs          # truck helpers
│   ├── solver/
│   │   └── tabu_search/         # metaheuristic implementation
│   │       ├── search.rs        # main search loop
│   │       ├── neighborhood.rs  # move generation
│   │       └── tabu.rs          # tabu list logic
│   ├── evaluation/              # scoring
│   │   ├── fitness.rs
│   │   └── penalty.rs
│   ├── setup/                   # instance/distance-matrix build
│   │   └── init.rs
│   ├── api/                     # OSRM/Google helpers
│   ├── database/                # SQLite pool
│   ├── config.rs                # constants
│   └── test/input_generator.rs  # synthetic inputs
├── scripts/visualize.py         # plot best_so_far.csv
└── best_so_far.csv              # solver output sample
```

## Dependencies
- Rust toolchain (1.70+ recommended) with Cargo.
- Docker + OSRM backend (required for distances) running and reachable.
- SQLite (for token cache used by `sqlx`).
- Python 3 with `pandas` and `matplotlib` for plotting.

## Run
1. Install Rust and ensure `cargo` is on PATH.
2. Start an OSRM backend in Docker (required distance source).
3. Build/run the solver: `cargo run --bin vrp-solver`
4. After a run, plot results: `python scripts/visualize.py`

Notes
- Logging via `tracing`; set verbosity with `RUST_LOG=info cargo run --bin vrp-solver`.
- Distance source is configured in [src/setup/init.rs](src/setup/init.rs); defaults to OSRM.