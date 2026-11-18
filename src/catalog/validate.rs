//! Checks the whole catalog against the conventions in
//! `docs/catalog-format.md`.

use std::path::Path;

use super::catalog::Catalog;
use super::conventions::is_valid_id;
use super::model::{CatalogEntry, SourceKind};
use crate::validation::Issue;

pub fn validate(catalog: &Catalog) -> Vec<Issue> {
  let mut issues = Vec::new();

  for entry in catalog.all() {
    check_entry(entry, catalog, &mut issues);
  }

  issues
}

fn check_entry(entry: &CatalogEntry, catalog: &Catalog, issues: &mut Vec<Issue>) {
  let location = entry.id.clone();
  let path = Path::new(&entry.source.file);

  if !is_valid_id(&entry.id) {
    issues.push(Issue::error(
      &location,
      "IDs may only use lowercase letters, digits and single dashes (for example 'kde-plasma')",
    ));
  }

  if entry.name.trim().is_empty() {
    issues.push(Issue::error(&location, "'name' is empty"));
  }

  if entry.description.trim().is_empty() {
    issues.push(Issue::warning(&location, "'description' is empty"));
  }

  // File name must be `<id>.yml`.
  let stem = path
    .file_stem()
    .and_then(|stem| stem.to_str())
    .unwrap_or("");
  if stem != entry.id {
    issues.push(Issue::error(
      &location,
      format!(
        "file '{}' should be named '{}.yml' to match its ID",
        entry.source.file, entry.id
      ),
    ));
  }

  // The folder should match the category, e.g. `compositors/`.
  let folder = path
    .parent()
    .and_then(|parent| parent.file_name())
    .and_then(|name| name.to_str())
    .unwrap_or("");
  let expected = entry.category.folder_name();
  if folder != expected {
    let message = format!(
      "category is '{}', so the file belongs in a '{expected}/' folder",
      entry.category.id()
    );

    // Built-in entries must follow the rule; for user entries it is a tip.
    match entry.source.kind {
      SourceKind::Builtin => issues.push(Issue::error(&location, message)),
      SourceKind::User => issues.push(Issue::warning(&location, message)),
    }
  }

  if entry.installation.package_managers.is_empty() {
    issues.push(Issue::warning(
      &location,
      "no installation information (add packages for at least 'arch')",
    ));
  }

  for (index, dependency) in entry.dependencies.iter().enumerate() {
    if dependency == &entry.id {
      issues.push(Issue::error(
        format!("{location}.dependencies[{index}]"),
        "an entry cannot depend on itself",
      ));
    } else if catalog.get(dependency).is_none() {
      // A warning for now: the catalog is still small. Milestone 3 turns
      // this into an error once the dependencies have their own entries.
      issues.push(Issue::warning(
        format!("{location}.dependencies[{index}]"),
        format!("'{dependency}' is not a catalog entry yet"),
      ));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::validation::{Severity, count};

  #[test]
  fn builtin_catalog_has_no_errors() {
    let catalog = Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap();
    let issues = validate(&catalog);
    assert_eq!(count(&issues, Severity::Error), 0, "{issues:#?}");
  }
}
