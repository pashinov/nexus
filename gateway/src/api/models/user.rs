use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DevicesRequest {
    pub r#type: String,
}

#[derive(Debug, Serialize)]
pub struct Device {
    pub id: String,
    pub r#type: String,
}
