"""
Visualize VRP routes on a Folium map.

Inputs:
- Routes JSON file (either an array of routes or an object with key "routes").
  Each route is a list of postal codes, e.g. [["207224", "529538", ...], ["207224", "769093", ...]].
- Customers CSV (default: data/customers.csv) with columns postal_code,demand.

Environment:
- ONE_MAP_EMAIL and ONE_MAP_PASS must be set for OneMap token retrieval.

Usage (with uv):
    uv pip install folium requests
    uv run scripts/visualize_routes.py routes.json --output map.html

Optional flags:
    --customers data/customers.csv   # override customers CSV path
    --cache data/geo_cache.json      # JSON cache for geocoded coordinates
    --warehouse 207224               # warehouse postal code for centering
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

import folium
import requests
from dotenv import load_dotenv

ONEMAP_TOKEN_URL = "https://www.onemap.gov.sg/api/auth/post/getToken"

ONEMAP_SEARCH_URL = (
    "https://www.onemap.gov.sg/api/common/elastic/search"
    "?searchVal={postal}&returnGeom=Y&getAddrDetails=Y&pageNum=1"
)


def load_routes(path: Path) -> List[List[str]]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, dict) and "routes" in data:
        routes = data["routes"]
    else:
        routes = data
    if not isinstance(routes, list):
        raise ValueError("Routes JSON must be a list or an object with key 'routes'.")
    normalized = []
    for idx, route in enumerate(routes):
        if not isinstance(route, list):
            raise ValueError(f"Route {idx} is not a list")
        normalized.append([str(p).strip() for p in route if str(p).strip()])
    return normalized


def load_customers(path: Path) -> Dict[str, int]:
    demands: Dict[str, int] = {}
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            parts = [p.strip() for p in line.split(",")]
            if not parts or not parts[0] or parts[0] == "postal_code":
                continue
            postal = parts[0]
            demand = int(parts[1]) if len(parts) > 1 and parts[1] else 0
            demands[postal] = demand
    return demands


def get_onemap_token() -> str:
    load_dotenv()
    email = os.environ.get("ONE_MAP_EMAIL")
    password = os.environ.get("ONE_MAP_PASS")
    if not email or not password:
        raise RuntimeError(
            "ONE_MAP_EMAIL and ONE_MAP_PASS must be set in the environment"
        )
    resp = requests.post(
        ONEMAP_TOKEN_URL, json={"email": email, "password": password}, timeout=10
    )
    resp.raise_for_status()
    data = resp.json()
    return data["access_token"]


def geocode_postal(postal: str, token: str) -> Tuple[float, float]:
    url = ONEMAP_SEARCH_URL.format(postal=postal)
    resp = requests.get(url, headers={"Authorization": f"Bearer {token}"}, timeout=10)
    resp.raise_for_status()
    data = resp.json()
    results = data.get("results") or []
    if not results:
        raise RuntimeError(f"No geocode result for postal {postal}")
    lat = float(results[0]["LATITUDE"])
    lon = float(results[0]["LONGITUDE"])
    return lat, lon


def load_cache(path: Path) -> Dict[str, Tuple[float, float]]:
    if path.exists():
        with path.open("r", encoding="utf-8") as f:
            raw = json.load(f)
            return {k: tuple(v) for k, v in raw.items()}
    return {}


def save_cache(path: Path, cache: Dict[str, Tuple[float, float]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        json.dump(cache, f, indent=2)


def ensure_coords(
    postals: Iterable[str], token: str, cache: Dict[str, Tuple[float, float]]
):
    missing = []
    for postal in postals:
        if postal not in cache:
            missing.append(postal)
    for postal in missing:
        cache[postal] = geocode_postal(postal, token)
    return cache


def add_route_layers(
    fmap: folium.Map,
    routes: List[List[str]],
    coords: Dict[str, Tuple[float, float]],
    demands: Dict[str, int],
    warehouse: str,
):
    palette = [
        "#1f77b4",
        "#ff7f0e",
        "#2ca02c",
        "#d62728",
        "#9467bd",
        "#8c564b",
        "#e377c2",
        "#7f7f7f",
        "#bcbd22",
        "#17becf",
    ]

    for idx, route in enumerate(routes):
        color = palette[idx % len(palette)]
        poly_points: List[Tuple[float, float]] = []
        visit_order = 1

        for postal in route:
            lat, lon = coords[postal]
            demand = demands.get(postal, 0)
            is_depot = postal == warehouse

            if is_depot:
                popup = f"Depot: {postal}"
                tooltip = popup
                radius = 7
            else:
                popup = f"#{visit_order}: {postal} — {demand / 1000:.0f}k"
                tooltip = popup
                radius = 5
                visit_order += 1

            folium.CircleMarker(
                location=(lat, lon),
                radius=radius,
                color=color,
                fill=True,
                fill_color=color,
                popup=popup,
                tooltip=tooltip,
            ).add_to(fmap)

            poly_points.append((lat, lon))

        folium.PolyLine(
            poly_points,
            color=color,
            weight=4,
            opacity=0.8,
            tooltip=f"Truck {idx + 1}",
        ).add_to(fmap)


def main() -> None:
    parser = argparse.ArgumentParser(description="Visualize VRP routes on a Folium map")

    parser.add_argument(
        "--routes",
        type=Path,
        default=Path("../data/routes.json"),
        help="Path to routes JSON file",
    )
    parser.add_argument(
        "--customers",
        type=Path,
        default=Path("../data/customers.csv"),
        help="Customers CSV path",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("routes_map.html"),
        help="Output HTML map path",
    )

    parser.add_argument(
        "--cache",
        type=Path,
        default=Path("../data/geo_cache.json"),
        help="Geocode cache file",
    )
    parser.add_argument(
        "--warehouse",
        type=str,
        default="207224",
        help="Warehouse postal code for centering",
    )
    args = parser.parse_args()

    routes = load_routes(args.routes)
    demands = load_customers(args.customers)

    token = get_onemap_token()
    cache = load_cache(args.cache)

    all_postals = set(p for route in routes for p in route)
    cache = ensure_coords(all_postals, token, cache)
    save_cache(args.cache, cache)

    # Center map on warehouse or first postal
    center_postal = args.warehouse or next(iter(all_postals))
    if center_postal not in cache:
        cache = ensure_coords([center_postal], token, cache)
        save_cache(args.cache, cache)
    center_lat, center_lon = cache[center_postal]

    fmap = folium.Map(
        location=(center_lat, center_lon), zoom_start=12, control_scale=True
    )
    add_route_layers(fmap, routes, cache, demands, args.warehouse)

    fmap.save(str(args.output))
    print(f"Saved map to {args.output}")


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:  # noqa: BLE001
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)
