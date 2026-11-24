//! What DotForge remembers between runs: which catalog entries it manages
//! for each profile. That is how it notices that something was *removed*
//! from a profile, so it can ask what to do with it.
//!
//! Stored as JSON in `~/.local/state/dotforge/state.json`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const STATE_FILE: &str = "state.json";
const VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct State {
  pub version: u32,
  /// Keyed by the full path of the profile's `profile.yml`.
  #[serde(default)]
  pub profiles: BTreeMap<String, ProfileState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProfileState {
  /// Keyed by catalog ID.
  #[serde(default)]
  pub entries: BTreeMap<String, ManagedEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ManagedEntry {
  /// The packages this entry had when it was applied. Kept here so they can
  /// still be removed if the catalog entry changes or disappears later.
  #[serde(default)]
  pub packages: Vec<String>,
  #[serde(default)]
  pub aur: Vec<String>,
  /// `false` when everything was already installed before DotForge.
  #[serde(default)]
  pub installed_by_dotforge: bool,
}

pub fn state_file(state_dir: &Path) -> PathBuf {
  state_dir.join(STATE_FILE)
}

pub fn load(state_dir: &Path) -> Result<State, AppError> {
  let path = state_file(state_dir);
  if !path.exists() {
    return Ok(State {
      version: VERSION,
      ..State::default()
    });
  }

  let text = fs::read_to_string(&path).map_err(|source| AppError::Io {
    path: path.clone(),
    source,
  })?;

  serde_json::from_str(&text).map_err(|error| AppError::Invalid {
    message: format!("{}: {error}", path.display()),
    hint: Some(
      "this file is written by DotForge; if it got damaged, move it away and apply again".into(),
    ),
  })
}

pub fn save(state_dir: &Path, state: &State) -> Result<(), AppError> {
  fs::create_dir_all(state_dir).map_err(|source| AppError::Write {
    path: state_dir.to_path_buf(),
    source,
  })?;

  let path = state_file(state_dir);
  let temp = path.with_extension("json.tmp");
  let text = serde_json::to_string_pretty(state).expect("state always serializes");

  // Write to a temporary file first, then rename: an interrupted save can
  // never leave a half-written state file behind.
  fs::write(&temp, text + "\n").map_err(|source| AppError::Write {
    path: temp.clone(),
    source,
  })?;
  fs::rename(&temp, &path).map_err(|source| AppError::Write { path, source })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn missing_file_is_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(dir.path()).unwrap().profiles.is_empty());
  }

  #[test]
  fn save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = load(dir.path()).unwrap();
    state
      .profiles
      .entry("/p/profile.yml".into())
      .or_default()
      .entries
      .insert(
        "kitty".into(),
        ManagedEntry {
          packages: vec!["kitty".into()],
          aur: vec![],
          installed_by_dotforge: true,
        },
      );

    save(dir.path(), &state).unwrap();
    assert_eq!(load(dir.path()).unwrap(), state);
  }

  #[test]
  fn damaged_file_is_explained() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(state_file(dir.path()), "{ not json").unwrap();
    let error = load(dir.path()).unwrap_err();
    assert!(error.hint().unwrap().contains("move it away"));
  }
}
