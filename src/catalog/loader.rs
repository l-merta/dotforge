//! Reading catalog entries from YAML.

use std::fs;
use std::path::Path;

use include_dir::{Dir, include_dir};
use walkdir::WalkDir;

use super::model::{CatalogEntry, Source, SourceKind};
use crate::error::{AppError, parse_error};

/// The `catalog/` folder of the repository, copied into the program at
/// compile time. This means the catalog always matches the program version
/// and works no matter where `dotforge` is installed or run from.
static BUILTIN_CATALOG: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/catalog");

/// Parses one catalog entry from YAML text.
/// `file` is only used to make error messages point at the right file.
pub fn parse_entry(contents: &str, file: &str) -> Result<CatalogEntry, AppError> {
  let deserializer = serde_yaml::Deserializer::from_str(contents);

  serde_path_to_error::deserialize(deserializer).map_err(|error| parse_error(file, error))
}

/// Loads every entry of the built-in catalog.
pub fn load_builtin() -> Result<Vec<CatalogEntry>, AppError> {
  let mut entries = Vec::new();
  collect_builtin(&BUILTIN_CATALOG, &mut entries)?;
  entries.sort_by(|a, b| a.id.cmp(&b.id));
  Ok(entries)
}

fn collect_builtin(dir: &Dir<'_>, entries: &mut Vec<CatalogEntry>) -> Result<(), AppError> {
  for file in dir.files() {
    if !is_yaml(file.path()) {
      continue;
    }

    let name = file.path().display().to_string();
    let contents = file.contents_utf8().ok_or_else(|| AppError::Invalid {
      message: format!("built-in catalog file '{name}' is not valid UTF-8"),
      hint: None,
    })?;

    let mut entry = parse_entry(contents, &format!("catalog/{name}"))?;
    entry.source = Source {
      kind: SourceKind::Builtin,
      file: name,
    };
    entries.push(entry);
  }

  for subdir in dir.dirs() {
    collect_builtin(subdir, entries)?;
  }

  Ok(())
}

/// Loads every entry from a folder on disk (the user's catalog folder).
/// A missing folder simply means "no user entries".
pub fn load_from_dir(dir: &Path) -> Result<Vec<CatalogEntry>, AppError> {
  let mut entries = Vec::new();

  if !dir.exists() {
    return Ok(entries);
  }

  for item in WalkDir::new(dir).sort_by_file_name() {
    let item = item.map_err(|error| AppError::Io {
      path: error.path().unwrap_or(dir).to_path_buf(),
      source: error.into(),
    })?;

    if !item.file_type().is_file() || !is_yaml(item.path()) {
      continue;
    }

    let path = item.path();
    let contents = fs::read_to_string(path).map_err(|source| AppError::Io {
      path: path.to_path_buf(),
      source,
    })?;

    let file = path.display().to_string();
    let mut entry = parse_entry(&contents, &file)?;
    entry.source = Source {
      kind: SourceKind::User,
      file,
    };
    entries.push(entry);
  }

  Ok(entries)
}

fn is_yaml(path: &Path) -> bool {
  matches!(
    path.extension().and_then(|ext| ext.to_str()),
    Some("yml") | Some("yaml")
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builtin_catalog_loads() {
    let entries = load_builtin().expect("built-in catalog should load");
    assert!(entries.iter().any(|entry| entry.id == "hyprland"));
    assert!(
      entries
        .iter()
        .all(|entry| entry.source.kind == SourceKind::Builtin)
    );
  }

  #[test]
  fn parse_error_names_file_and_field() {
    let yaml = "id: x\nname: X\ndescription: d\ncategory: spaceship\ninstallation: {}\n";
    let error = parse_entry(yaml, "x.yml").unwrap_err();

    match &error {
      AppError::Parse {
        file, field, line, ..
      } => {
        assert_eq!(file, "x.yml");
        assert_eq!(field.as_deref(), Some("category"));
        assert_eq!(*line, Some(4));
      }
      other => panic!("expected a parse error, got {other:?}"),
    }
  }

  #[test]
  fn unknown_fields_are_rejected() {
    let yaml = "id: x\nname: X\ndescription: d\ncategory: shell\ninstallation: {}\ncolour: red\n";
    let error = parse_entry(yaml, "x.yml").unwrap_err().to_string();
    assert!(error.contains("colour"), "{error}");
  }

  #[test]
  fn missing_user_folder_is_empty() {
    let entries = load_from_dir(Path::new("/definitely/not/here")).unwrap();
    assert!(entries.is_empty());
  }
}
