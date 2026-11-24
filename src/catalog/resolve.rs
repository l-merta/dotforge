//! Working out dependencies: which entries are needed, and in which order
//! they should be installed (dependencies first).
//!
//! Dependencies are catalog IDs. Resolving walks them depth-first, the same
//! way you would by hand: "to install Hyprland I first need its portal, and
//! for the portal I first need xdg-desktop-portal".

use std::collections::{BTreeSet, HashMap};

use super::catalog::Catalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
  /// `id` is not in the catalog. `needed_by` is the entry that asked for
  /// it, or `None` when it was one of the starting IDs.
  Unknown {
    id: String,
    needed_by: Option<String>,
  },
  /// The entries depend on each other in a circle, e.g. `a → b → a`.
  Cycle(Vec<String>),
}

impl std::fmt::Display for ResolveError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ResolveError::Unknown {
        id,
        needed_by: None,
      } => write!(f, "'{id}' is not in the catalog"),
      ResolveError::Unknown {
        id,
        needed_by: Some(parent),
      } => write!(
        f,
        "'{parent}' depends on '{id}', which is not in the catalog"
      ),
      ResolveError::Cycle(path) => write!(f, "circular dependency: {}", path.join(" → ")),
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
  /// We are inside this entry's dependencies right now.
  Visiting,
  /// This entry and all its dependencies are already in the order.
  Done,
}

/// Returns every entry needed for `roots`, dependencies first, each ID once.
pub fn install_order(catalog: &Catalog, roots: &[&str]) -> Result<Vec<String>, ResolveError> {
  let mut marks = HashMap::new();
  let mut order = Vec::new();
  let mut path = Vec::new();

  for root in roots {
    visit(catalog, root, None, &mut marks, &mut path, &mut order)?;
  }

  Ok(order)
}

fn visit(
  catalog: &Catalog,
  id: &str,
  needed_by: Option<&str>,
  marks: &mut HashMap<String, Mark>,
  path: &mut Vec<String>,
  order: &mut Vec<String>,
) -> Result<(), ResolveError> {
  match marks.get(id) {
    Some(Mark::Done) => return Ok(()),
    Some(Mark::Visiting) => {
      // We came back to an entry we are still working on: a circle.
      let start = path.iter().position(|step| step == id).unwrap_or(0);
      let mut cycle: Vec<String> = path[start..].to_vec();
      cycle.push(id.to_string());
      return Err(ResolveError::Cycle(cycle));
    }
    None => {}
  }

  let Some(entry) = catalog.get(id) else {
    return Err(ResolveError::Unknown {
      id: id.to_string(),
      needed_by: needed_by.map(str::to_string),
    });
  };

  marks.insert(id.to_string(), Mark::Visiting);
  path.push(id.to_string());

  for dependency in &entry.dependencies {
    visit(catalog, dependency, Some(id), marks, path, order)?;
  }

  path.pop();
  marks.insert(id.to_string(), Mark::Done);
  order.push(id.to_string());
  Ok(())
}

/// Finds every circular dependency in the catalog, each reported once.
pub fn find_cycles(catalog: &Catalog) -> Vec<Vec<String>> {
  let mut seen = BTreeSet::new();
  let mut cycles = Vec::new();

  for entry in catalog.all() {
    if let Err(ResolveError::Cycle(cycle)) = install_order(catalog, &[&entry.id]) {
      // The same circle is found from each of its members; keep one.
      let key: BTreeSet<String> = cycle.iter().cloned().collect();
      if seen.insert(key.into_iter().collect::<Vec<_>>()) {
        cycles.push(cycle);
      }
    }
  }

  cycles
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::catalog::loader::parse_entry;
  use crate::catalog::model::{CatalogEntry, Source, SourceKind};

  fn entry(id: &str, deps: &[&str]) -> CatalogEntry {
    let yaml = format!(
      "id: {id}\nname: {id}\ndescription: t\ncategory: component\ninstallation: {{}}\ndependencies: [{}]\n",
      deps.join(", ")
    );
    let mut entry = parse_entry(&yaml, "t.yml").unwrap();
    entry.source = Source {
      kind: SourceKind::Builtin,
      file: format!("components/{id}.yml"),
    };
    entry
  }

  fn catalog(entries: Vec<CatalogEntry>) -> Catalog {
    Catalog::from_entries(entries, vec![]).unwrap()
  }

  #[test]
  fn dependencies_come_first_and_only_once() {
    let catalog = catalog(vec![
      entry("app", &["lib", "tool"]),
      entry("tool", &["lib"]),
      entry("lib", &[]),
    ]);

    assert_eq!(
      install_order(&catalog, &["app"]).unwrap(),
      ["lib", "tool", "app"]
    );
    assert_eq!(
      install_order(&catalog, &["tool", "app"]).unwrap(),
      ["lib", "tool", "app"]
    );
  }

  #[test]
  fn unknown_dependency_names_who_needs_it() {
    let catalog = catalog(vec![entry("app", &["ghost"])]);
    assert_eq!(
      install_order(&catalog, &["app"]).unwrap_err(),
      ResolveError::Unknown {
        id: "ghost".into(),
        needed_by: Some("app".into())
      }
    );
  }

  #[test]
  fn cycles_are_detected_once() {
    let catalog = catalog(vec![
      entry("a", &["b"]),
      entry("b", &["c"]),
      entry("c", &["a"]),
      entry("d", &[]),
    ]);

    let error = install_order(&catalog, &["a"]).unwrap_err();
    assert_eq!(
      error,
      ResolveError::Cycle(vec!["a".into(), "b".into(), "c".into(), "a".into()])
    );
    assert_eq!(find_cycles(&catalog).len(), 1);
  }

  #[test]
  fn builtin_catalog_has_no_cycles() {
    let catalog = Catalog::load_with_user_dir(std::path::Path::new("/nonexistent")).unwrap();
    assert!(find_cycles(&catalog).is_empty());
  }
}
