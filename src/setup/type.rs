/// Struct to match the JSON structure
#[derive(Debug, Deserialize)]
struct MRTLocation {
    #[serde(rename = "Possible Locations")]
    possible_locations: Vec<LocationData>,
}

#[derive(Debug, Deserialize)]
struct LocationData {
    #[serde(rename = "POSTAL")]
    postal: String,
}
