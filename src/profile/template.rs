//! Creating a new profile folder from a starter template.

use std::fs;
use std::path::{Path, PathBuf};

use super::loader::PROFILE_FILE;
use super::model::{CURRENT_FORMAT_VERSION, DeviceType};
use super::naming;
use crate::error::AppError;

/// Folders created inside every new profile.
pub const FOLDERS: [&str; 5] = [
  "config",
  "assets/wallpapers",
  "assets/icons",
  "assets/cursors",
  "assets/fonts",
];

pub struct NewProfile<'a> {
  /// Folder name, e.g. `desktop`.
  pub name: &'a str,
  /// Display name; defaults to the folder name.
  pub display_name: Option<&'a str>,
  pub description: Option<&'a str>,
  pub device_type: DeviceType,
}

/// Creates `<profiles_dir>/<name>/` with a `profile.yml` and the standard
/// folders. Refuses to touch an existing profile.
pub fn create(profiles_dir: &Path, new: &NewProfile) -> Result<PathBuf, AppError> {
  if !naming::is_valid(new.name) {
    let hint = match naming::suggest(new.name) {
      Some(suggestion) => format!("{}; try '{suggestion}'", naming::rules()),
      None => naming::rules(),
    };
    return Err(AppError::Invalid {
      message: format!("'{}' is not a valid profile name", new.name),
      hint: Some(hint),
    });
  }

  let folder = profiles_dir.join(new.name);
  if folder.exists() {
    return Err(AppError::AlreadyExists {
      what: "profile",
      name: new.name.to_string(),
      hint: Some(format!(
        "it is at '{}'; pick another name or edit that profile",
        folder.display()
      )),
    });
  }

  for sub in FOLDERS {
    let path = folder.join(sub);
    fs::create_dir_all(&path).map_err(|source| AppError::Write { path, source })?;
  }

  let file = folder.join(PROFILE_FILE);
  fs::write(&file, render(new)).map_err(|source| AppError::Write {
    path: file.clone(),
    source,
  })?;

  Ok(folder)
}

/// The text of a new `profile.yml`. Only the required fields are filled in;
/// everything else is a commented-out example, so the new profile is valid
/// straight away and shows what can be added.
pub fn render(new: &NewProfile) -> String {
  let display_name = new.display_name.unwrap_or(new.name);
  let description = new.description.unwrap_or("");

  format!(
    r#"# DotForge profile. Field reference: docs/profile-format.md
# Check it any time with: dotforge profile validate {name}

format_version: {version}
name: {display_name}
description: {description}

device:
  type: {device_type}   # desktop, laptop or virtual-machine
  os:
    family: arch
    distribution: arch

# Remove the '#' in front of the lines you want to use.
# Every value is a catalog ID: see 'dotforge catalog list'.

# system:
#   audio:
#     backend: pipewire
#   graphics:
#     drivers: [mesa]
#     api: [vulkan]

# applications:
#   terminal: kitty
#   gaming: []
#   social: []
#   productivity: []
#   utilities: []
#   development: []
#   system: []

# rice:
#   compositor: hyprland
#   shell: noctalia
#   components:
#     notifications: noctalia
"#,
    name = new.name,
    version = CURRENT_FORMAT_VERSION,
    display_name = yaml_string(display_name),
    description = yaml_string(description),
    device_type = new.device_type,
  )
}

/// Quotes a string so any text (with `:` or `#` in it) stays valid YAML.
fn yaml_string(text: &str) -> String {
  serde_json::to_string(text).expect("strings always serialize")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::catalog::catalog::Catalog;
  use crate::profile::{loader, validate};

  fn new(name: &str) -> NewProfile<'_> {
    NewProfile {
      name,
      display_name: Some("My: \"fancy\" # desktop"),
      description: None,
      device_type: DeviceType::Laptop,
    }
  }

  #[test]
  fn template_is_a_valid_profile() {
    let text = render(&new("desk"));
    let profile = loader::parse(&text, "template").unwrap();
    assert_eq!(profile.name, "My: \"fancy\" # desktop");
    assert_eq!(profile.device.kind, DeviceType::Laptop);

    let catalog = Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap();
    assert!(validate::validate(&profile, &catalog).is_empty());
  }

  #[test]
  fn commented_examples_are_valid_too() {
    // Un-commenting the examples must also give a valid profile.
    let text: String = render(&new("desk"))
      .lines()
      .map(|line| {
        line
          .strip_prefix("# ")
          .filter(|_| {
            !line.contains("Remove the")
              && !line.contains("Every value")
              && !line.contains("DotForge profile")
              && !line.contains("Check it")
          })
          .unwrap_or(line)
      })
      .collect::<Vec<_>>()
      .join("\n");

    let profile = loader::parse(&text, "template").unwrap();
    let catalog = Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap();
    assert!(validate::validate(&profile, &catalog).is_empty());
    assert_eq!(profile.rice.compositor.as_deref(), Some("hyprland"));
  }

  #[test]
  fn creates_folders_and_refuses_to_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let folder = create(dir.path(), &new("desk")).unwrap();

    assert!(folder.join(PROFILE_FILE).is_file());
    for sub in FOLDERS {
      assert!(folder.join(sub).is_dir(), "{sub}");
    }

    assert!(matches!(
      create(dir.path(), &new("desk")),
      Err(AppError::AlreadyExists { .. })
    ));
  }

  #[test]
  fn rejects_bad_names() {
    let dir = tempfile::tempdir().unwrap();
    let error = create(dir.path(), &new("My Desk")).unwrap_err();
    assert!(error.hint().unwrap().contains("'my-desk'"));
    assert!(!dir.path().join("My Desk").exists());
  }
}
