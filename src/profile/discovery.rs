//! Finding profiles in the profiles folder *without* fully loading them.
//!
//! Listing should stay fast and should still work when one profile is
//! broken, so this only reads each folder name and a few header fields
//! (`name`, `description`). Full checking is `dotforge profile validate`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::loader::PROFILE_FILE;
use super::naming;
use crate::error::AppError;
use crate::validation::Issue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
  /// The folder has a `profile.yml` whose header could be read.
  Ok,
  /// The folder has no `profile.yml`.
  MissingFile,
  /// `profile.yml` exists but could not be read as YAML.
  Unreadable,
}

#[derive(Debug, Clone, Serialize)]
pub struct FoundProfile {
  /// The folder name, used in commands (`dotforge profile show <name>`).
  pub name: String,
  /// The `name:` field from `profile.yml`, if it could be read.
  pub display_name: Option<String>,
  pub description: Option<String>,
  pub path: String,
  pub status: Status,
  /// What is wrong, when the status is not `ok`.
  pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Discovery {
  pub profiles_dir: String,
  pub profiles: Vec<FoundProfile>,
  /// Things worth fixing, such as two profiles with the same display name.
  pub issues: Vec<Issue>,
}

/// Only the header fields; everything else in the file is ignored here.
#[derive(Deserialize)]
struct Header {
  name: Option<serde_yaml::Value>,
  description: Option<serde_yaml::Value>,
}

pub fn discover(profiles_dir: &Path) -> Result<Discovery, AppError> {
  let mut profiles = Vec::new();
  let mut issues = Vec::new();

  if profiles_dir.is_dir() {
    let entries = fs::read_dir(profiles_dir).map_err(|source| AppError::Io {
      path: profiles_dir.to_path_buf(),
      source,
    })?;

    for entry in entries {
      let entry = entry.map_err(|source| AppError::Io {
        path: profiles_dir.to_path_buf(),
        source,
      })?;
      let path = entry.path();

      // Hidden folders (like `.git`) and loose files are not profiles.
      let folder_name = entry.file_name().to_string_lossy().to_string();
      if !path.is_dir() || folder_name.starts_with('.') {
        continue;
      }

      profiles.push(inspect(&folder_name, &path));
    }
  }

  profiles.sort_by(|a, b| a.name.cmp(&b.name));

  check_names(&profiles, &mut issues);

  Ok(Discovery {
    profiles_dir: profiles_dir.display().to_string(),
    profiles,
    issues,
  })
}

fn inspect(folder_name: &str, path: &Path) -> FoundProfile {
  let file = path.join(PROFILE_FILE);
  let mut found = FoundProfile {
    name: folder_name.to_string(),
    display_name: None,
    description: None,
    path: path.display().to_string(),
    status: Status::Ok,
    problem: None,
  };

  if !file.is_file() {
    found.status = Status::MissingFile;
    found.problem = Some(format!("the folder has no {PROFILE_FILE}"));
    return found;
  }

  let header = fs::read_to_string(&file)
    .map_err(|error| error.to_string())
    .and_then(|text| serde_yaml::from_str::<Header>(&text).map_err(|error| error.to_string()));

  match header {
    Ok(header) => {
      found.display_name = header.name.as_ref().and_then(as_text);
      found.description = header.description.as_ref().and_then(as_text);
    }
    Err(error) => {
      found.status = Status::Unreadable;
      found.problem = Some(error);
    }
  }

  found
}

fn as_text(value: &serde_yaml::Value) -> Option<String> {
  match value {
    serde_yaml::Value::String(text) => Some(text.clone()),
    serde_yaml::Value::Number(number) => Some(number.to_string()),
    _ => None,
  }
}

fn check_names(profiles: &[FoundProfile], issues: &mut Vec<Issue>) {
  // Folder names that break the naming rules cannot be typed reliably.
  for profile in profiles {
    if !naming::is_valid(&profile.name) {
      let tip = match naming::suggest(&profile.name) {
        Some(suggestion) => format!("; rename the folder to '{suggestion}'"),
        None => String::new(),
      };
      issues.push(Issue::warning(
        &profile.name,
        format!("{}{tip}", naming::rules()),
      ));
    }
  }

  // Two folders that only differ in upper/lower case, e.g. `Desktop` and
  // `desktop`, are easy to mix up.
  let mut by_lowercase: BTreeMap<String, Vec<&str>> = BTreeMap::new();
  for profile in profiles {
    by_lowercase
      .entry(profile.name.to_lowercase())
      .or_default()
      .push(&profile.name);
  }
  for names in by_lowercase.values().filter(|names| names.len() > 1) {
    issues.push(Issue::warning(
      names.join(", "),
      "these profile folders only differ in upper/lower case",
    ));
  }

  // Two profiles showing the same display name look identical in lists.
  let mut by_display: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
  for profile in profiles {
    if let Some(display) = &profile.display_name {
      by_display
        .entry(display.as_str())
        .or_default()
        .push(&profile.name);
    }
  }
  for (display, names) in by_display.iter().filter(|(_, names)| names.len() > 1) {
    issues.push(Issue::warning(
      names.join(", "),
      format!("these profiles all use the display name '{display}'"),
    ));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::validation::{Severity, count};

  fn write(dir: &Path, folder: &str, contents: Option<&str>) {
    let folder = dir.join(folder);
    fs::create_dir_all(&folder).unwrap();
    if let Some(contents) = contents {
      fs::write(folder.join(PROFILE_FILE), contents).unwrap();
    }
  }

  #[test]
  fn missing_folder_means_no_profiles() {
    let result = discover(Path::new("/definitely/not/here")).unwrap();
    assert!(result.profiles.is_empty());
  }

  #[test]
  fn finds_profiles_and_reports_problems() {
    let dir = tempfile::tempdir().unwrap();
    write(
      dir.path(),
      "desktop",
      Some("name: Desk\ndescription: Main PC\n"),
    );
    write(dir.path(), "empty", None);
    write(dir.path(), "broken", Some("name: [unclosed\n"));
    write(dir.path(), ".git", None);
    fs::write(dir.path().join("notes.txt"), "hi").unwrap();

    let result = discover(dir.path()).unwrap();
    let names: Vec<&str> = result.profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["broken", "desktop", "empty"]);

    assert_eq!(result.profiles[0].status, Status::Unreadable);
    assert_eq!(result.profiles[1].status, Status::Ok);
    assert_eq!(result.profiles[1].display_name.as_deref(), Some("Desk"));
    assert_eq!(result.profiles[2].status, Status::MissingFile);
  }

  #[test]
  fn listing_does_not_need_a_valid_profile() {
    // Missing required fields: a full load would fail, listing must not.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "half", Some("name: Half done\n"));

    let result = discover(dir.path()).unwrap();
    assert_eq!(result.profiles[0].status, Status::Ok);
  }

  #[test]
  fn warns_about_duplicates_and_bad_names() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a", Some("name: Same\n"));
    write(dir.path(), "b", Some("name: Same\n"));
    write(dir.path(), "My Laptop", Some("name: Laptop\n"));

    let result = discover(dir.path()).unwrap();
    assert_eq!(
      count(&result.issues, Severity::Warning),
      2,
      "{:#?}",
      result.issues
    );
    assert!(
      result
        .issues
        .iter()
        .any(|i| i.message.contains("'my-laptop'"))
    );
  }
}
