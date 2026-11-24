//! DotForge's own settings, read from `~/.config/dotforge/config.yml`.
//!
//! The file is optional. Every setting has a default, so a missing file just
//! means "use the defaults". Unknown settings are rejected so typos are
//! caught.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, parse_error};
use crate::paths;

pub const SETTINGS_FILE: &str = "config.yml";
pub const DEFAULT_EDITOR: &str = "nano";

/// The settings as written in the file. `None` means "not set, use the
/// default".
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
  /// The program used to open files, e.g. `nano` or `code --wait`.
  #[serde(default)]
  pub editor: Option<String>,
}

impl Settings {
  /// The editor command to use: the setting, or `nano`.
  pub fn editor(&self) -> &str {
    self
      .editor
      .as_deref()
      .map(str::trim)
      .filter(|editor| !editor.is_empty())
      .unwrap_or(DEFAULT_EDITOR)
  }
}

/// `~/.config/dotforge/config.yml`
pub fn settings_file() -> Result<PathBuf, AppError> {
  Ok(paths::config_dir()?.join(SETTINGS_FILE))
}

/// Loads the settings file, or the defaults if there is none.
pub fn load() -> Result<Settings, AppError> {
  load_from(&settings_file()?)
}

pub fn load_from(path: &Path) -> Result<Settings, AppError> {
  if !path.exists() {
    return Ok(Settings::default());
  }

  let contents = fs::read_to_string(path).map_err(|source| AppError::Io {
    path: path.to_path_buf(),
    source,
  })?;

  parse(&contents, &path.display().to_string())
}

pub fn parse(contents: &str, file: &str) -> Result<Settings, AppError> {
  // An empty file (or one with only comments) means "all defaults".
  if serde_yaml::from_str::<serde_yaml::Value>(contents)
    .map(|value| value.is_null())
    .unwrap_or(false)
  {
    return Ok(Settings::default());
  }

  serde_path_to_error::deserialize(serde_yaml::Deserializer::from_str(contents))
    .map_err(|error| parse_error(file, error))
}

/// The text written by `dotforge config edit` when there is no file yet.
pub const TEMPLATE: &str = "# DotForge settings. Every setting is optional.
# See 'dotforge config show' for the settings in use.

# The program used by 'dotforge profile edit' and 'dotforge config edit'.
# Examples: nano, vim, micro, code --wait, kate -b
editor: nano
";

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn missing_or_empty_file_means_defaults() {
    assert_eq!(
      load_from(Path::new("/nope/config.yml")).unwrap().editor(),
      "nano"
    );
    assert_eq!(parse("", "c.yml").unwrap().editor(), "nano");
    assert_eq!(
      parse("# only a comment\n", "c.yml").unwrap().editor(),
      "nano"
    );
    assert_eq!(parse("editor: \"  \"\n", "c.yml").unwrap().editor(), "nano");
  }

  #[test]
  fn editor_setting_is_used() {
    assert_eq!(
      parse("editor: code --wait\n", "c.yml").unwrap().editor(),
      "code --wait"
    );
  }

  #[test]
  fn typos_are_errors() {
    let error = parse("editr: vim\n", "c.yml").unwrap_err().to_string();
    assert!(error.contains("editr"), "{error}");
  }

  #[test]
  fn template_is_valid() {
    assert_eq!(parse(TEMPLATE, "t").unwrap().editor(), "nano");
  }
}
