use std::path::Path;

use walkdir::WalkDir;

use super::loader;
use super::model::{CatalogEntry, Category};

pub struct Catalog {
  entries: Vec<CatalogEntry>,
}

impl Catalog {
  pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(path) {
      let entry = entry?;

      if entry.path().extension().and_then(|ext| ext.to_str()) != Some("yml") {
        continue;
      }

      let catalog_entry = loader::load_entry(entry.path())?;

      entries.push(catalog_entry);
    }

    Ok(Self { entries })
  }

  pub fn all(&self) -> &[CatalogEntry] {
    &self.entries
  }

  pub fn get(&self, id: &str) -> Option<&CatalogEntry> {
    self.entries.iter().find(|entry| entry.id == id)
  }

  pub fn by_category(&self, category: &Category) -> Vec<&CatalogEntry> {
    self.entries
      .iter()
      .filter(|entry| &entry.category == category)
      .collect()
  }
}