use futures::future::join_all;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use std::sync::Arc;
use tokio::task;
use tracing::error;

const FACTOR: usize = 10;

pub async fn create_dm_google(
    locations: Vec<String>,
    num_of_trucks: usize,
    api_key: &str,
) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
    let num_locations = locations.len();
    let client = Arc::new(Client::new());
    let (padded_locations, m_count) = pad_locations(locations);

    let mut tasks = Vec::new();
    for m_col in 0..m_count {
        let destinations = &padded_locations[m_col * FACTOR..(m_col + 1) * FACTOR];
        for m_row in 0..m_count {
            let origins = &padded_locations[m_row * FACTOR..(m_row + 1) * FACTOR];
            let client_clone = Arc::clone(&client);
            let api_key_ref = api_key.to_string();
            let destinations_vec = destinations.to_vec();
            let origins_vec = origins.to_vec();
            tasks.push(task::spawn(async move {
                match get_google_single_dm(
                    client_clone,
                    &origins_vec,
                    &destinations_vec,
                    &api_key_ref,
                )
                .await
                {
                    Ok(matrix) => matrix,
                    Err(e) => {
                        error!("Error fetching distance matrix: {}", e);
                        vec![vec![0.0; FACTOR]; FACTOR]
                    }
                }
            }));
        }
    }

    let results = join_all(tasks).await;

    let mut dm: Option<Vec<Vec<f64>>> = None;
    let mut results_iter = results.into_iter();

    for _m_col in 0..m_count {
        let mut temp_col = vec![];
        for _m_row in 0..m_count {
            if let Ok(matrix) = results_iter.next().unwrap() {
                temp_col.extend(matrix);
            }
        }

        match dm {
            None => dm = Some(temp_col),
            Some(ref mut dm_matrix) => {
                for (i, row) in dm_matrix.iter_mut().enumerate() {
                    row.extend(temp_col[i].iter());
                }
            }
        }
    }

    let mut dm = dm.expect("Distance matrix should have been initialized");
    dm.truncate(num_locations);
    dm.iter_mut().for_each(|row| row.truncate(num_locations));

    if num_of_trucks > 1 {
        let partition_counter = num_of_trucks - 2;
        let truck_row = vec![0.0; partition_counter]
            .into_iter()
            .chain(dm[0].clone())
            .collect::<Vec<f64>>();
        let truck_rows = vec![truck_row.clone(); partition_counter];

        for row in dm.iter_mut() {
            row.splice(0..0, std::iter::repeat_n(row[0], partition_counter));
        }

        dm.splice(0..0, truck_rows.into_iter());
    }

    Ok(dm)
}

fn pad_locations(locations: Vec<String>) -> (Vec<String>, usize) {
    let num_locations = locations.len();
    let last_set = num_locations % FACTOR;
    let padding_needed = FACTOR - last_set;

    let mut padded_locations = locations.to_owned();

    if last_set > 0 {
        let first_location = locations[0].clone();
        let padding: Vec<String> = std::iter::repeat(first_location)
            .take(padding_needed)
            .collect();
        padded_locations.extend(padding);
    }

    let m_count = padded_locations.len() / FACTOR;
    (padded_locations, m_count)
}

pub async fn get_google_single_dm(
    client: Arc<Client>,
    origins: &[String],
    destinations: &[String],
    api_key: &str,
) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
    let base_url = "https://maps.googleapis.com/maps/api/distancematrix/json";
    let url = format!(
        "{}?origins={}&destinations={}&key={}",
        base_url,
        origins.join("|"),
        destinations.join("|"),
        api_key
    );

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<DistanceMatrixResponse>()
        .await?;

    let mut dist_matrix = Vec::new();
    for row in response.rows {
        let mut row_data = Vec::new();
        for element in row.elements {
            if let Some(distance) = element.distance {
                row_data.push(distance.value as f64 / 1000.0);
            } else {
                row_data.push(0.0);
            }
        }
        dist_matrix.push(row_data);
    }

    Ok(dist_matrix)
}

#[derive(Debug, Deserialize)]
struct DistanceMatrixResponse {
    rows: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    elements: Vec<Element>,
}

#[derive(Debug, Deserialize)]
struct Element {
    distance: Option<Distance>,
}

#[derive(Debug, Deserialize)]
struct Distance {
    value: i32,
}
