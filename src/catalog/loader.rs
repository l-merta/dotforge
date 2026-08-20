use std::fs;
use std::path::Path;

use super::model::CatalogEntry;

pub fn load_entry(path: &Path) -> Result<CatalogEntry, Box<dyn std::error::Error>> {
  let contents = fs::read_to_string(path)?;

  let entry: CatalogEntry = serde_yaml::from_str(&contents)?;

  Ok(entry)
}