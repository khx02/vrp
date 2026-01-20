# VRP Solver

Vehicle Routing Problem (VRP) solver written in Rust. It builds distance matrices (OSRM/Google), generates initial solutions, and runs a tabu-search–style metaheuristic with diversification to find good routes. Features early stopping based on stagnation, multiple diversification strategies, and outputs progress to `best_so_far.csv` with final solution metrics (distance, fitness, penalty).

## Project Layout
```
├── src/
│   ├── bin/
│   │   └── vrp-solver.rs        # thin binary entrypoint
│   ├── lib.rs                   # library surface (re-exports)
│   ├── domain/                  # core data types
│   │   ├── types.rs             # Location, Route, Truck, ProblemInstance, MRTLocation
│   │   └── solution.rs          # truck helpers
│   ├── distance/                # distance matrix orchestration
│   │   ├── matrix.rs            # orchestrator (provider selection)
│   │   ├── mod.rs
│   │   └── providers/
│   │       ├── mod.rs
│   │       ├── google.rs        # Google Maps API
│   │       └── osrm.rs          # OSRM + OneMap token caching
│   ├── solver/
│   │   ├── mod.rs
│   │   └── tabu_search/
│   │       ├── mod.rs
│   │       ├── search.rs        # main search loop (early stopping, metrics)
│   │       ├── neighbourhood.rs # move generation (2-swap)
│   │       ├── tabu.rs          # tabu list & aspiration
│   │       ├── diversification.rs  # rollback, mutation, steer
│   │       └── repair.rs        # ALNS destroy & recreate
│   ├── evaluation/              # scoring & penalties
│   │   ├── mod.rs
│   │   ├── fitness.rs
│   │   └── penalty.rs
│   ├── setup/                   # instance/distance-matrix build
│   │   ├── mod.rs
│   │   └── init.rs
│   ├── database/                # SQLite pool & token cache
│   │   ├── mod.rs
│   │   └── sqlx.rs
│   ├── fixtures/                # test data generators
│   │   ├── mod.rs
│   │   └── data_generator.rs
│   ├── config.rs                # constants (DISTANCE_PROVIDER, etc.)
│   ├── lib.rs                   # library entry point
│   ├── main.rs                  # binary entry point
│   └── utils.rs                 # utility functions
├── osrm/                        # OSRM backend (docker)
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

## `.env` Example

Create a `.env` file in the project root with the following variables:

```env
# Distance provider: "osrm" or "google"
DISTANCE_PROVIDER=osrm

# Google Maps API key (required if DISTANCE_PROVIDER = "google")
GOOGLE_API_KEY=your_google_api_key_here

# OneMap credentials (required for OSRM provider)
ONE_MAP_EMAIL=your_onemap_email@example.com
ONE_MAP_PASS=your_onemap_password

# OSRM endpoint for Multi-Level Dijkstra (MLD) (the docker is setted up to forward port 6000 to 5000)
OSRM_BASE_URL=http://localhost:6000/table/v1/driving
```