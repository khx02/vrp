use serde::Deserialize;

/// Struct to match the JSON structure
#[derive(Debug, Deserialize)]
pub struct MRTLocation {
    #[serde(rename = "Possible Locations")]
    pub possible_locations: Vec<LocationData>,
}

#[derive(Debug, Deserialize)]
pub struct LocationData {
    #[serde(rename = "POSTAL")]
    pub postal: String,
}
