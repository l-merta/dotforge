//! Reading `/etc/os-release`, the standard file every modern Linux
//! distribution uses to say what it is.
//!
//! Example (EndeavourOS):
//!
//! ```text
//! NAME="EndeavourOS"
//! ID="endeavouros"
//! ID_LIKE="arch"
//! ```

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::profile::model::OsFamily;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OsRelease {
  /// Short lowercase ID, e.g. `arch`, `endeavouros`, `ubuntu`.
  pub id: String,
  /// IDs of distributions this one is based on, e.g. `["arch"]`.
  pub id_like: Vec<String>,
  /// Readable name, e.g. `Arch Linux`.
  pub name: String,
  pub version_id: Option<String>,
}

/// Reads `<root>/etc/os-release`, falling back to `<root>/usr/lib/os-release`.
pub fn read(root: &Path) -> Option<OsRelease> {
  ["etc/os-release", "usr/lib/os-release"]
    .iter()
    .find_map(|file| fs::read_to_string(root.join(file)).ok())
    .map(|text| parse(&text))
}

pub fn parse(text: &str) -> OsRelease {
  let mut release = OsRelease::default();
  let mut pretty_name = None;

  for line in text.lines() {
    let Some((key, value)) = line.trim().split_once('=') else {
      continue;
    };
    let value = unquote(value.trim());

    match key {
      "ID" => release.id = value.to_lowercase(),
      "ID_LIKE" => {
        release.id_like = value.split_whitespace().map(str::to_lowercase).collect();
      }
      "NAME" => release.name = value,
      "PRETTY_NAME" => pretty_name = Some(value),
      "VERSION_ID" => release.version_id = Some(value),
      _ => {}
    }
  }

  if release.name.is_empty() {
    release.name = pretty_name.unwrap_or_else(|| release.id.clone());
  }

  release
}

fn unquote(value: &str) -> String {
  let trimmed = value
    .strip_prefix('"')
    .and_then(|v| v.strip_suffix('"'))
    .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
    .unwrap_or(value);
  trimmed.replace("\\\"", "\"")
}

/// Which DotForge OS family a distribution belongs to.
///
/// A distribution is in the Arch family when it *is* Arch, or says it is
/// "like" Arch (EndeavourOS, Manjaro, CachyOS, Garuda, ...).
pub fn family_of(release: &OsRelease) -> Option<OsFamily> {
  let is = |id: &str| release.id == id || release.id_like.iter().any(|like| like == id);

  if is("arch") {
    Some(OsFamily::Arch)
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_arch() {
    let release =
      parse("NAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\nBUILD_ID=rolling\n");
    assert_eq!(release.id, "arch");
    assert_eq!(release.name, "Arch Linux");
    assert_eq!(family_of(&release), Some(OsFamily::Arch));
  }

  #[test]
  fn arch_based_distributions_are_arch_family() {
    let release = parse("NAME='EndeavourOS'\nID=\"endeavouros\"\nID_LIKE=\"arch\"\n");
    assert_eq!(release.name, "EndeavourOS");
    assert_eq!(family_of(&release), Some(OsFamily::Arch));

    let manjaro = parse("ID=manjaro\nID_LIKE=\"arch\"\n");
    assert_eq!(family_of(&manjaro), Some(OsFamily::Arch));
  }

  #[test]
  fn other_distributions_have_no_family_yet() {
    let release = parse("NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\nVERSION_ID=\"24.04\"\n");
    assert_eq!(release.version_id.as_deref(), Some("24.04"));
    assert_eq!(family_of(&release), None);
  }

  #[test]
  fn ignores_comments_and_junk() {
    let release = parse("# a comment\n\nID=arch\nnot a pair\n");
    assert_eq!(release.id, "arch");
    assert_eq!(release.name, "arch");
  }
}
