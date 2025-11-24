use reqwest::Client;

// Standard library imports
use serde_json::Value;


// 1. OSRM Distance Matrix Function
pub async fn create_dm_osrm(coords: &[(f64, f64)]) -> Option<Vec<Vec<f64>>> {
    if coords.is_empty() {
        eprintln!("create_dm_osrm: coords are empty");
        return None;
    }

    // Format coords as "lon,lat;lon,lat;..."
    let coord_str = coords
        .iter()
        // OSRM wants longitude,latitude in that order
        .map(|(lat, lon)| format!("{},{}", lon, lat))
        .collect::<Vec<String>>()
        .join(";");

    let base_url = "https://router.project-osrm.org/table/v1/driving";
    let url = format!("{}/{}?annotations=distance", base_url, coord_str);

    let client = Client::new();
    println!("create_dm_osrm: sending GET to {url}");

    // Make the request
    let response = match client.get(&url)
        .header("User-Agent", "VRP/1.0 (denzylcs@gmail.com)")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("create_dm_osrm: request failed: {e}");
            return None;
        }
    };

    // Convert response to text
    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("create_dm_osrm: failed to read response body: {e}");
            return None;
        }
    };

    // Parse JSON
    let json: Value = match serde_json::from_str(&text) {
        Ok(js) => js,
        Err(e) => {
            eprintln!("create_dm_osrm: failed to parse JSON: {e}");
            return None;
        }
    };

    let distances = json["distances"].as_array()?;
    // Convert distances (in meters) to kilometers
    let matrix = distances
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|val| val.as_f64().unwrap_or(f64::MAX) / 1000.0)
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();

    Some(matrix)
}

// Convert a List of Locations (Postal Codes) into (lat, lon) Using OneMap
pub async fn convert_to_coords(locations: Vec<String>) -> Vec<(f64, f64)> {
    let mut coords = vec![];

    // Example token (DON'T commit real tokens in production):
    let onemap_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...ORxI9A";

    for pc in &locations {
        match get_coordinates_from_postal(pc, onemap_token).await {
            Some((lat, lon)) => coords.push((lat, lon)),
            None => eprintln!("Could not find coordinates for postal code: {pc}"),
        }
    }
    coords
}

/// Attempt to fetch latitude/longitude for a Singapore postal code using 
/// OneMap's Search API. You must already have a valid access token.
async fn get_coordinates_from_postal(
    postal_code: &str,
    access_token: &str,
) -> Option<(f64, f64)> {
    // 1. Construct the OneMap "Search" endpoint with postal code
    // Example: https://www.onemap.gov.sg/api/common/elastic/search?searchVal=200640&returnGeom=Y&getAddrDetails=Y&pageNum=1
    let url = format!(
        "https://www.onemap.gov.sg/api/common/elastic/search?searchVal={}&returnGeom=Y&getAddrDetails=Y&pageNum=1",
        postal_code
    );

    let client = Client::new();
    println!("get_coordinates_from_postal: sending GET to {url}");

    // 2. Send request with Authorization header
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

    // 3. Convert response to text
    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to read response body: {e}");
            return None;
        }
    };

    // 4. Parse JSON
    let json: Value = match serde_json::from_str(&text) {
        Ok(js) => js,
        Err(e) => {
            eprintln!("JSON parse error: {e}");
            eprintln!("Raw response: {}", text);
            return None;
        }
    };

    // Typically, OneMap's "Search" returns something like:
    // {
    //   "found": 1,
    //   "results": [
    //       {
    //         "SEARCHVAL": "200640",
    //         "LATITUDE": "1.310023",
    //         "LONGITUDE": "103.862367",
    //         ...
    //       }
    //   ]
    // }
    let results = json["results"].as_array()?;
    if results.is_empty() {
        eprintln!("No results found for postal code: {postal_code}");
        return None;
    }

    // 5. Extract lat/lon from the first result
    let lat = results[0]["LATITUDE"].as_str()?.parse::<f64>().ok()?;
    let lon = results[0]["LONGITUDE"].as_str()?.parse::<f64>().ok()?;

    Some((lat, lon))
}


