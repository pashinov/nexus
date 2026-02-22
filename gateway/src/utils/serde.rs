use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

pub fn load_json_from_file<T, P>(path: P) -> Result<T>
where
    for<'de> T: Deserialize<'de>,
    P: AsRef<Path>,
{
    let data = std::fs::read_to_string(path)?;
    let de = &mut serde_json::Deserializer::from_str(&data);
    serde_path_to_error::deserialize(de).map_err(Into::into)
}
