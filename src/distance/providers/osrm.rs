use chrono::Utc;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::env;
use std::error::Error;
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Deserialize)]
struct OneMapTokenResponse {
    access_token: String,
    expiry_timestamp: String,
}

async fn fetch_onemap_token() -> Result<(String, i64), Box<dyn Error>> {
    dotenv().ok();
    let email = env::var("ONE_MAP_EMAIL")?;
    let password = env::var("ONE_MAP_PASS")?;
    let url = "https://www.onemap.gov.sg/api/auth/post/getToken";
    let client = Client::new();
    let payload = serde_json::json!({
        "email": email,
        "password": password
    });
    trace!("Fetching OneMap token from {}", url);
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;
    if !response.status().is_success() {
        error!("OneMap auth failed with status: {}", response.status());
        return Err(format!("OneMap auth failed: {}", response.status()).into());
    }
    let json: OneMapTokenResponse = response.json().await?;
    let expiry_timestamp: i64 = json.expiry_timestamp.parse()?;
    info!("Successfully obtained OneMap access token");
    Ok((json.access_token, expiry_timestamp))
}

async fn get_onemap_token(pool: &SqlitePool) -> Result<String, Box<dyn Error>> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT token, expiry FROM api_tokens WHERE service = 'onemap'")
            .fetch_optional(pool)
            .await?;
    let current_time = Utc::now().timestamp();
    if let Some((token, expiry)) = row {
        if current_time < expiry {
            info!("Using cached OneMap token from DB");
            return Ok(token);
        } else {
            info!("Cached token expired ({}), fetching new one", expiry);
        }
    } else {
        info!("No existing OneMap token in DB, fetching first time");
    }
    let (new_token, expiry_timestamp) = fetch_onemap_token().await?;
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO api_tokens (service, token, expiry)
        VALUES ('onemap', ?, ?)
        "#,
    )
    .bind(&new_token)
    .bind(expiry_timestamp)
    .execute(pool)
    .await?;
    info!(
        "Stored new OneMap token in DB with expiry: {}",
        expiry_timestamp
    );
    Ok(new_token)
}

pub async fn create_dm_osrm(coords: &[(f64, f64)]) -> Option<Vec<Vec<f64>>> {
    dotenv().ok();
    if coords.is_empty() {
        error!("create_dm_osrm: coords are empty");
        return None;
    }
    let base_url = env::var("OSRM_BASE_URL")
        .unwrap_or_else(|_| "https://router.project-osrm.org/table/v1/driving".to_string());
    let is_public_osrm = base_url.contains("router.project-osrm.org");
    let coord_str = coords
        .iter()
        .map(|(lat, lon)| format!("{},{}", lon, lat))
        .collect::<Vec<String>>()
        .join(";");
    let url = format!("{}/{}?annotations=distance", base_url, coord_str);
    if url.len() > 8000 {
        warn!(
            "OSRM URL too long ({} chars), consider self-hosted OSRM or batching",
            url.len()
        );
        return None;
    }
    trace!("Formatted coordinate string: {}", coord_str);
    debug!("Built OSRM URL: {} ({} chars)", url, url.len());
    let client = Client::new();
    info!("Sending GET request to OSRM ({} locations)", coords.len());
    let mut request_builder = client.get(&url);
    if is_public_osrm {
        let user_agent = env::var("ONE_MAP_EMAIL")
            .map(|email| format!("VRP-Solver/1.0 ({})", email.trim()))
            .unwrap_or_else(|_| "VRP-Solver/1.0 (no-email-configured@example.com)".to_string());
        request_builder = request_builder.header("User-Agent", &user_agent);
        info!("Using public OSRM — added User-Agent: {}", &user_agent);
    } else {
        info!("Using local/self-hosted OSRM — no User-Agent header required");
    }
    let response = match request_builder
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            debug!(
                "Received response: HTTP {} ({} bytes)",
                status,
                resp.content_length().unwrap_or(0)
            );
            if !status.is_success() {
                error!(
                    "OSRM returned HTTP {}: {}",
                    status,
                    status.canonical_reason().unwrap_or("Unknown")
                );
                return None;
            }
            resp
        }
        Err(e) => {
            error!("OSRM request failed: {} (coords: {})", e, coords.len());
            if e.to_string().contains("handshake") || e.to_string().contains("TLS") {
                warn!(
                    "TLS/handshake failure — likely blocked by public OSRM. Switch to self-hosted."
                );
            }
            return None;
        }
    };
    let text = match response.text().await {
        Ok(t) => {
            trace!("Response size: {} bytes", t.len());
            if t.contains("too many locations") || t.contains("request too large") {
                warn!(
                    "OSRM rejected request due to too many locations ({})",
                    coords.len()
                );
                return None;
            }
            t
        }
        Err(e) => {
            error!("Failed to read OSRM response body: {}", e);
            return None;
        }
    };
    let json: Value = match serde_json::from_str::<Value>(&text) {
        Ok(js) => {
            debug!(
                "Successfully parsed JSON ({} objects)",
                js.as_object().map_or(0, |o| o.len())
            );
            js
        }
        Err(e) => {
            error!(
                "Failed to parse OSRM JSON: {} (first 200 chars: {})",
                e,
                &text[..text.len().min(200)]
            );
            return None;
        }
    };
    let distances = match json["distances"].as_array() {
        Some(arr) => {
            info!(
                "Extracted {}x{} distances array from OSRM",
                arr.len(),
                arr[0].as_array().map_or(0, |r| r.len())
            );
            arr
        }
        None => {
            error!(
                "No 'distances' array in OSRM response. Keys: {:?}",
                json.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
            return None;
        }
    };
    let matrix = distances
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let row_len = row.as_array().map_or(0, |r| r.len());
            debug!("Processing row {} ({} cols)", row_idx, row_len);
            row.as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|val| {
                    let dist_km = val.as_f64().unwrap_or(f64::MAX) / 1000.0;
                    if dist_km >= f64::MAX / 2.0 {
                        trace!("Unreachable distance in row {}: {:.0}", row_idx, dist_km);
                    }
                    dist_km
                })
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();
    info!(
        "Successfully created distance matrix: {}x{} ({} locations)",
        matrix.len(),
        matrix[0].len(),
        coords.len()
    );
    Some(matrix)
}

pub async fn convert_to_coords(pool: &SqlitePool, locations: Vec<String>) -> Vec<(f64, f64)> {
    let mut coords = vec![];
    let token: String = match get_onemap_token(pool).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get OneMap token: {}", e);
            return vec![];
        }
    };
    for pc in &locations {
        match get_coordinates_from_postal(&pc, &token).await {
            Some((lat, lon)) => coords.push((lat, lon)),
            None => eprintln!("Could not find coordinates for postal code: {}", pc),
        }
    }
    info!("coords: {:?}", &coords);
    coords
}

async fn get_coordinates_from_postal(postal_code: &str, access_token: &str) -> Option<(f64, f64)> {
    let url = format!(
        "https://www.onemap.gov.sg/api/common/elastic/search?searchVal={}&returnGeom=Y&getAddrDetails=Y&pageNum=1",
        postal_code
    );
    let client = Client::new();
    trace!("get_coordinates_from_postal: sending GET to {url}");
    let response = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Request failed for {postal_code}: {e}");
            return None;
        }
    };
    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to read response body: {e}");
            return None;
        }
    };
    let json: Value = match serde_json::from_str(&text) {
        Ok(js) => js,
        Err(e) => {
            eprintln!("JSON parse error: {e}");
            eprintln!("Raw response: {}", text);
            return None;
        }
    };
    let results = json["results"].as_array()?;
    if results.is_empty() {
        eprintln!("No results found for postal code: {postal_code}");
        return None;
    }
    let lat = results[0]["LATITUDE"].as_str()?.parse::<f64>().ok()?;
    let lon = results[0]["LONGITUDE"].as_str()?.parse::<f64>().ok()?;
    Some((lat, lon))
}
