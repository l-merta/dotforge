//! The combined catalog: built-in entries plus the user's own entries.

use std::collections::BTreeMap;
use std::path::Path;

use super::loader;
use super::model::{CatalogEntry, Category};
use crate::error::AppError;

pub struct Catalog {
  /// Sorted by ID.
  entries: Vec<CatalogEntry>,
  /// IDs of built-in entries that a user entry replaced.
  overridden: Vec<String>,
}

impl Catalog {
  /// Loads the built-in catalog and the user catalog folder
  /// (`~/.config/dotforge/catalog/`).
  pub fn load() -> Result<Self, AppError> {
    let user_dir = crate::paths::user_catalog_dir()?;
    Self::load_with_user_dir(&user_dir)
  }

  /// Same as [`Catalog::load`], but with a custom user folder (used in tests).
  pub fn load_with_user_dir(user_dir: &Path) -> Result<Self, AppError> {
    let builtin = loader::load_builtin()?;
    let user = loader::load_from_dir(user_dir)?;
    Self::from_entries(builtin, user)
  }

  /// Combines built-in and user entries.
  ///
  /// Two entries with the same ID from the *same* source are an error,
  /// because DotForge could not know which one you meant. A user entry with
  /// the same ID as a built-in entry replaces it on purpose.
  pub fn from_entries(
    builtin: Vec<CatalogEntry>,
    user: Vec<CatalogEntry>,
  ) -> Result<Self, AppError> {
    let mut by_id: BTreeMap<String, CatalogEntry> = BTreeMap::new();
    let mut overridden = Vec::new();

    for entry in builtin.into_iter().chain(user) {
      match by_id.get(&entry.id) {
        Some(existing) if existing.source.kind == entry.source.kind => {
          return Err(AppError::Invalid {
            message: format!(
              "catalog ID '{}' is used twice: in {} and in {}",
              entry.id, existing.source, entry.source
            ),
            hint: Some("every catalog entry needs its own unique ID".into()),
          });
        }
        Some(_) => overridden.push(entry.id.clone()),
        None => {}
      }

      by_id.insert(entry.id.clone(), entry);
    }

    Ok(Self {
      entries: by_id.into_values().collect(),
      overridden,
    })
  }

  pub fn all(&self) -> &[CatalogEntry] {
    &self.entries
  }

  pub fn get(&self, id: &str) -> Option<&CatalogEntry> {
    self.entries.iter().find(|entry| entry.id == id)
  }

  pub fn by_category(&self, category: Category) -> Vec<&CatalogEntry> {
    self
      .entries
      .iter()
      .filter(|entry| entry.category == category)
      .collect()
  }

  /// IDs of built-in entries that were replaced by a user entry.
  pub fn overridden(&self) -> &[String] {
    &self.overridden
  }

  /// Finds IDs that look similar to `id`, to suggest "did you mean ...?".
  pub fn similar_ids(&self, id: &str) -> Vec<&str> {
    self
      .entries
      .iter()
      .map(|entry| entry.id.as_str())
      .filter(|candidate| is_similar(id, candidate))
      .collect()
  }
}

/// Very small "did you mean" check: one contains the other, or they differ
/// by at most two letters.
fn is_similar(a: &str, b: &str) -> bool {
  if a.len() >= 3 && (a.contains(b) || b.contains(a)) {
    return true;
  }

  edit_distance(a, b) <= 2
}

fn edit_distance(a: &str, b: &str) -> usize {
  let a: Vec<char> = a.chars().collect();
  let b: Vec<char> = b.chars().collect();
  let mut previous: Vec<usize> = (0..=b.len()).collect();

  for i in 1..=a.len() {
    let mut current = vec![i; b.len() + 1];

    for j in 1..=b.len() {
      let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
      current[j] = (previous[j] + 1)
        .min(current[j - 1] + 1)
        .min(previous[j - 1] + cost);
    }

    previous = current;
  }

  previous[b.len()]
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::catalog::loader::parse_entry;
  use crate::catalog::model::{Source, SourceKind};

  fn entry(id: &str, kind: SourceKind) -> CatalogEntry {
    let yaml = format!(
      "id: {id}\nname: {id}\ndescription: test\ncategory: application\ninstallation: {{}}\n"
    );
    let mut entry = parse_entry(&yaml, "test.yml").unwrap();
    entry.source = Source {
      kind,
      file: format!("{id}.yml"),
    };
    entry
  }

  #[test]
  fn lookup_and_filter() {
    let catalog = Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap();
    assert_eq!(catalog.get("niri").unwrap().category, Category::Compositor);
    assert!(catalog.get("does-not-exist").is_none());
    assert!(
      catalog
        .by_category(Category::Compositor)
        .iter()
        .all(|entry| entry.category == Category::Compositor)
    );
  }

  #[test]
  fn user_entry_overrides_builtin() {
    let catalog = Catalog::from_entries(
      vec![entry("kitty", SourceKind::Builtin)],
      vec![entry("kitty", SourceKind::User)],
    )
    .unwrap();

    assert_eq!(catalog.all().len(), 1);
    assert_eq!(catalog.get("kitty").unwrap().source.kind, SourceKind::User);
    assert_eq!(catalog.overridden(), ["kitty".to_string()]);
  }

  #[test]
  fn duplicate_ids_in_same_source_fail() {
    let result = Catalog::from_entries(
      vec![],
      vec![
        entry("kitty", SourceKind::User),
        entry("kitty", SourceKind::User),
      ],
    );
    assert!(result.is_err());
  }

  #[test]
  fn suggests_similar_ids() {
    let catalog =
      Catalog::from_entries(vec![entry("hyprland", SourceKind::Builtin)], vec![]).unwrap();
    assert_eq!(catalog.similar_ids("hyprlnd"), vec!["hyprland"]);
    assert!(catalog.similar_ids("firefox").is_empty());
  }
}
