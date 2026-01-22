"""
Generate random Singapore postal codes for VRP customer locations, including a demand column.
Saves them to data/customers.csv with header `postal_code,demand`.

Usage:
    python scripts/generate_random_customers.py [count] [--output path]

Examples:
    python3 scripts/generate_random_customers.py 75
    python scripts/generate_random_customers.py 100 --output data/my_customers.csv
"""

import argparse
import csv
import json
import random
from pathlib import Path


def load_mrt_postal_codes(path: Path, max_count: int) -> list[str]:
    """Load up to `max_count` postal codes from mrt_data JSON file."""
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)

    seen = set()
    postals: list[str] = []

    for entry in data:
        locations = entry.get("Possible Locations", [])
        if not locations:
            continue
        postal = locations[0].get("POSTAL")
        if not postal:
            continue
        postal = str(postal).strip()
        if len(postal) != 6 or not postal.isdigit():
            continue
        if postal in seen:
            continue
        seen.add(postal)
        postals.append(postal)
        if len(postals) >= max_count:
            break

    return postals


def resolve_output_path(path_str: str, repo_root: Path) -> Path:
    """Resolve output path relative to repo root unless absolute is provided."""
    path = Path(path_str).expanduser()
    if path.is_absolute():
        return path
    return repo_root / path


def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent  # scripts/ -> repo root
    default_output = repo_root / "data" / "customers.csv"
    default_mrt = repo_root / "mrt_data.json"
    default_seed = 64
    default_demand_min = 450000
    default_demand_max = 500000

    parser = argparse.ArgumentParser(
        description="Generate random customer postal codes for VRP",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "count", type=int, help="Number of customer locations to generate"
    )
    parser.add_argument(
        "--output",
        type=str,
        default=str(default_output.relative_to(repo_root)),
        help="Output CSV file path (relative to repo root unless absolute)",
    )
    parser.add_argument(
        "--mrt-path",
        type=str,
        default=str(default_mrt.relative_to(repo_root)),
        help="Path to mrt_data.json (relative to repo root unless absolute)",
    )
    parser.add_argument(
        "--demand-min",
        type=int,
        default=default_demand_min,
        help="Minimum customer demand value (inclusive)",
    )
    parser.add_argument(
        "--demand-max",
        type=int,
        default=default_demand_max,
        help="Maximum customer demand value (inclusive)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=default_seed,
        help="Random seed for deterministic postal ordering and demand values",
    )

    args = parser.parse_args()

    if args.demand_min <= 0 or args.demand_max < args.demand_min:
        raise ValueError(
            "Invalid demand range; ensure max >= min and both are positive."
        )

    mrt_path = resolve_output_path(args.mrt_path, repo_root)
    print(f"Loading up to {args.count} customer postal codes from {mrt_path}...")
    postals = load_mrt_postal_codes(mrt_path, args.count)

    rng = random.Random(args.seed)

    output_path = resolve_output_path(args.output, repo_root)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with output_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["postal_code", "demand"])
        for postal in postals:
            demand = rng.randint(args.demand_min, args.demand_max)
            writer.writerow([postal, demand])

    print(
        f"Done! Saved {len(postals)} customer rows (with demand) to: {output_path} using seed {args.seed}"
    )


if __name__ == "__main__":
    main()
