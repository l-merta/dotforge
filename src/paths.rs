//! Where DotForge looks for the user's own files.
//!
//! These paths never depend on the folder you run `dotforge` from.
//!
//! The config folder is chosen like this (first match wins):
//! 1. `$DOTFORGE_CONFIG_DIR` (useful for testing)
//! 2. `$XDG_CONFIG_HOME/dotforge`
//! 3. `$HOME/.config/dotforge`

use std::env;
use std::path::PathBuf;

use crate::error::AppError;

pub fn config_dir() -> Result<PathBuf, AppError> {
  if let Some(dir) = non_empty_env("DOTFORGE_CONFIG_DIR") {
    return Ok(PathBuf::from(dir));
  }

  if let Some(dir) = non_empty_env("XDG_CONFIG_HOME") {
    return Ok(PathBuf::from(dir).join("dotforge"));
  }

  if let Some(home) = non_empty_env("HOME") {
    return Ok(PathBuf::from(home).join(".config").join("dotforge"));
  }

  Err(AppError::NoConfigDir)
}

/// `~/.config/dotforge/profiles/`
pub fn profiles_dir() -> Result<PathBuf, AppError> {
  Ok(config_dir()?.join("profiles"))
}

/// `~/.config/dotforge/catalog/`
pub fn user_catalog_dir() -> Result<PathBuf, AppError> {
  Ok(config_dir()?.join("catalog"))
}

/// Where DotForge keeps what it remembers between runs (which entries it
/// manages, and logs): `$DOTFORGE_STATE_DIR`, else
/// `$XDG_STATE_HOME/dotforge`, else `~/.local/state/dotforge`.
pub fn state_dir() -> Result<PathBuf, AppError> {
  if let Some(dir) = non_empty_env("DOTFORGE_STATE_DIR") {
    return Ok(PathBuf::from(dir));
  }

  if let Some(dir) = non_empty_env("XDG_STATE_HOME") {
    return Ok(PathBuf::from(dir).join("dotforge"));
  }

  if let Some(home) = non_empty_env("HOME") {
    return Ok(PathBuf::from(home).join(".local/state/dotforge"));
  }

  Err(AppError::NoConfigDir)
}

/// Your home folder (`$HOME`). Config files and assets are linked into it.
pub fn home_dir() -> Result<PathBuf, AppError> {
  non_empty_env("HOME")
    .map(PathBuf::from)
    .ok_or(AppError::NoConfigDir)
}

fn non_empty_env(name: &str) -> Option<String> {
  env::var(name).ok().filter(|value| !value.is_empty())
}
