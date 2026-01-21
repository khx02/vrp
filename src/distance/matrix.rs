use sqlx::SqlitePool;
use tracing::{error, info};

use super::providers::{convert_to_coords, create_dm_google, create_dm_osrm};

/// Create distance matrix from the specified provider (google or osrm)
pub async fn create_dm(
    source: &str,
    locations: Vec<String>,
    num_of_trucks: usize,
    api_key: Option<&str>,
    pool: SqlitePool,
) -> Vec<Vec<f64>> {
    info!(
        "Creating distance matrix using source '{}' ({} locations, {} trucks)",
        source,
        locations.len(),
        num_of_trucks
    );

    match source {
        "google" => {
            let api_key = api_key.expect("API key required for Google source");
            match create_dm_google(locations, num_of_trucks, api_key).await {
                Ok(matrix) => {
                    info!("Successfully retrieved matrix from Google API");
                    matrix
                }
                Err(e) => {
                    error!("Google API request failed: {:?}", e);
                    vec![vec![]]
                }
            }
        }

        "osrm" => {
            let mut target_locations = locations;

            if num_of_trucks > 1 {
                let warehouse = target_locations[0].clone();
                target_locations.splice(0..0, std::iter::repeat_n(warehouse, num_of_trucks - 2));
            }

            let coords = convert_to_coords(&pool, target_locations).await;

            if coords.len() < 2 {
                error!("Insufficient valid coordinates for distance matrix");
                return vec![vec![]];
            }

            match create_dm_osrm(&coords).await {
                Some(matrix) => {
                    info!("Successfully retrieved matrix from OSRM");
                    matrix
                }
                None => {
                    error!("OSRM failed to return a valid distance matrix");
                    vec![vec![]]
                }
            }
        }

        _ => {
            error!("Unknown distance matrix source: {}", source);
            vec![vec![]]
        }
    }
}
