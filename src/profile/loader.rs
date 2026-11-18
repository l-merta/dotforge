//! Finding and reading `profile.yml` files.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::model::{CURRENT_FORMAT_VERSION, Profile};
use crate::error::{AppError, parse_error};

pub const PROFILE_FILE: &str = "profile.yml";

/// Works out which file a user means.
///
/// - Something that looks like a path (`./desktop`, `profiles/test`,
///   `~/x/profile.yml`) is used as a path. A folder means
///   `<folder>/profile.yml`.
/// - A plain name (`desktop`) means `~/.config/dotforge/profiles/desktop/`.
pub fn resolve(name_or_path: &str, profiles_dir: &Path) -> Result<PathBuf, AppError> {
  let looks_like_path = name_or_path.contains('/') || name_or_path.ends_with(".yml");

  let path = if looks_like_path {
    PathBuf::from(name_or_path)
  } else {
    profiles_dir.join(name_or_path)
  };

  let file = if path.is_dir() {
    path.join(PROFILE_FILE)
  } else {
    path
  };

  if file.is_file() {
    return Ok(file);
  }

  let hint = if looks_like_path {
    format!(
      "expected a profile folder or a '{PROFILE_FILE}' file at '{}'",
      file.display()
    )
  } else {
    format!(
      "profiles are looked up in '{}'; you can also pass a path, e.g. './my-profile'",
      profiles_dir.display()
    )
  };

  Err(AppError::NotFound {
    what: "profile",
    name: name_or_path.to_string(),
    hint: Some(hint),
  })
}

/// Reads and parses a `profile.yml` file.
pub fn load(path: &Path) -> Result<Profile, AppError> {
  let contents = fs::read_to_string(path).map_err(|source| AppError::Io {
    path: path.to_path_buf(),
    source,
  })?;

  parse(&contents, &path.display().to_string())
}

/// Only the version field, read first so that a profile written for a newer
/// DotForge gets a clear "please update" message instead of confusing
/// errors about unknown fields.
#[derive(Deserialize)]
struct VersionOnly {
  format_version: Option<serde_yaml::Value>,
}

/// Parses profile YAML. `file` is only used in error messages.
pub fn parse(contents: &str, file: &str) -> Result<Profile, AppError> {
  let header: VersionOnly =
    serde_path_to_error::deserialize(serde_yaml::Deserializer::from_str(contents))
      .map_err(|error| parse_error(file, error))?;

  match header
    .format_version
    .as_ref()
    .and_then(|value| value.as_u64())
  {
    None => {
      return Err(AppError::Invalid {
        message: format!("{file}: 'format_version' is missing or not a whole number"),
        hint: Some(format!(
          "add 'format_version: {CURRENT_FORMAT_VERSION}' at the top of the profile"
        )),
      });
    }
    Some(version) if version != CURRENT_FORMAT_VERSION as u64 => {
      let hint = if version > CURRENT_FORMAT_VERSION as u64 {
        "this profile was written for a newer DotForge; please update DotForge"
      } else {
        "this profile uses an old format that is no longer supported"
      };

      return Err(AppError::Invalid {
        message: format!(
          "{file}: profile format version {version} is not supported (this DotForge reads version {CURRENT_FORMAT_VERSION})"
        ),
        hint: Some(hint.into()),
      });
    }
    Some(_) => {}
  }

  serde_path_to_error::deserialize(serde_yaml::Deserializer::from_str(contents))
    .map_err(|error| parse_error(file, error))
}

#[cfg(test)]
mod tests {
  use super::*;

  const MINIMAL: &str = "format_version: 1\nname: Mini\ndevice:\n  type: laptop\n  os:\n    family: arch\n    distribution: arch\n";

  #[test]
  fn minimal_profile_parses() {
    let profile = parse(MINIMAL, "p.yml").unwrap();
    assert_eq!(profile.name, "Mini");
    assert!(profile.rice.compositor.is_none());
    assert!(profile.applications.gaming.is_empty());
  }

  #[test]
  fn repo_test_profile_parses() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles/test/profile.yml");
    load(&path).expect("profiles/test/profile.yml should parse");
  }

  #[test]
  fn missing_version_is_explained() {
    let error = parse("name: x\n", "p.yml").unwrap_err();
    assert!(error.to_string().contains("format_version"));
    assert!(error.hint().unwrap().contains("format_version: 1"));
  }

  #[test]
  fn newer_version_asks_to_update() {
    let error = parse("format_version: 99\nwhatever: true\n", "p.yml").unwrap_err();
    assert!(error.hint().unwrap().contains("newer DotForge"));
  }

  #[test]
  fn invalid_value_names_the_field() {
    let yaml = MINIMAL.replace("family: arch", "family: windows");
    let error = parse(&yaml, "p.yml").unwrap_err();

    match error {
      AppError::Parse {
        field,
        line,
        message,
        ..
      } => {
        assert_eq!(field.as_deref(), Some("device.os.family"));
        assert_eq!(line, Some(6));
        assert_eq!(message, "unknown variant `windows`, expected `arch`");
      }
      other => panic!("expected a parse error, got {other:?}"),
    }
  }

  #[test]
  fn missing_required_field() {
    let yaml = MINIMAL.replace("name: Mini\n", "");
    let error = parse(&yaml, "p.yml").unwrap_err().to_string();
    assert!(error.contains("missing field `name`"), "{error}");
  }

  #[test]
  fn typo_in_field_name_is_caught() {
    let yaml = format!("{MINIMAL}aplications:\n  terminal: kitty\n");
    let error = parse(&yaml, "p.yml").unwrap_err().to_string();
    assert!(error.contains("aplications"), "{error}");
  }

  #[test]
  fn resolve_by_name_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let profile_dir = dir.path().join("desktop");
    std::fs::create_dir(&profile_dir).unwrap();
    std::fs::write(profile_dir.join(PROFILE_FILE), MINIMAL).unwrap();

    let by_name = resolve("desktop", dir.path()).unwrap();
    assert_eq!(by_name, profile_dir.join(PROFILE_FILE));

    let path = profile_dir.display().to_string();
    assert_eq!(resolve(&path, Path::new("/elsewhere")).unwrap(), by_name);

    assert!(matches!(
      resolve("laptop", dir.path()),
      Err(AppError::NotFound { .. })
    ));
  }
}
