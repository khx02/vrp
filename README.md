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
│   │       ├── search.rs        # main search loop
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
├── scripts/visualize.py         # plot best_so_far.csv
├── mrt_data.json                # MRT postal codes
├── osrm-sg/                     # OSRM data files
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
- Distance provider is configured in [src/config.rs](src/config.rs#L9) (`DISTANCE_PROVIDER`): set to `"osrm"` (default) or `"google"`.
- If using Google Maps, add `GOOGLE_API_KEY=your_key` to `.env` file.
- If using OSRM, ensure `ONE_MAP_EMAIL` and `ONE_MAP_PASS` are in `.env` for OneMap token retrieval (required to convert postal codes to coordinates).

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

# OSRM endpoint (optional, defaults to public OSRM at https://router.project-osrm.org)
# Set this to use a local OSRM Docker instance instead:
OSRM_BASE_URL=http://localhost:5000/table/v1/driving
```

**Notes:**
- For OSRM (default), only `ONE_MAP_EMAIL` and `ONE_MAP_PASS` are required.
- By default, OSRM uses the **public OSRM service** (`https://router.project-osrm.org`).
- To use the locally hosted OSRM (in `osrm-sg/`), provide `OSRM_BASE_URL=http://localhost:5000/table/v1/driving` (adjust port as needed).
- For Google Maps, provide your API key and set `DISTANCE_PROVIDER=google`.