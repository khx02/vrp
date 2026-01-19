"""
Generate random Singapore postal codes for VRP customer locations.
Saves them to data/customers.csv (one postal code per line, no header by default).

Usage:
    python scripts/generate_random_customers.py [count] [--output path]

Examples:
    python3 scripts/generate_random_customers.py 75
    python scripts/generate_random_customers.py 100 --output data/my_customers.csv
"""

import argparse
import csv
import json
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

    args = parser.parse_args()

    mrt_path = resolve_output_path(args.mrt_path, repo_root)
    print(f"Loading up to {args.count} customer postal codes from {mrt_path}...")
    postals = load_mrt_postal_codes(mrt_path, args.count)

    output_path = resolve_output_path(args.output, repo_root)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with output_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        # Uncomment to add a header row:
        # writer.writerow(["postal_code"])
        for postal in postals:
            writer.writerow([postal])

    print(f"Done! Saved {len(postals)} postal codes to: {output_path}")


if __name__ == "__main__":
    main()
