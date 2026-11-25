//! Checks a parsed profile against the catalog.
//!
//! Parsing (in `loader.rs`) already guarantees that required fields exist and
//! that fixed values such as `device.type` are valid. This file checks the
//! things YAML cannot: that every ID points to a real catalog entry of the
//! right kind.

use std::collections::HashSet;

use super::model::Profile;
use crate::catalog::catalog::Catalog;
use crate::catalog::model::{Capability, Category, Protocol};
use crate::validation::Issue;

pub fn validate(profile: &Profile, catalog: &Catalog) -> Vec<Issue> {
  let mut issues = Vec::new();

  if profile.name.trim().is_empty() {
    issues.push(Issue::error("name", "the profile name is empty"));
  }

  if profile.device.os.distribution.trim().is_empty() {
    issues.push(Issue::error(
      "device.os.distribution",
      "the distribution is empty",
    ));
  }

  // Applications
  let apps = &profile.applications;
  let mut seen = HashSet::new();

  if let Some(terminal) = &apps.terminal {
    check_id(
      catalog,
      "applications.terminal",
      terminal,
      &[Category::Application],
      &mut issues,
    );
  }

  for (group, ids) in apps.groups() {
    for (index, id) in ids.iter().enumerate() {
      let location = format!("applications.{group}[{index}]");
      check_id(
        catalog,
        &location,
        id,
        &[Category::Application],
        &mut issues,
      );

      if !seen.insert(id.as_str()) {
        issues.push(Issue::warning(
          location,
          format!("'{id}' is listed more than once"),
        ));
      }
    }
  }

  // Rice
  let rice = &profile.rice;
  if let Some(id) = &rice.desktop {
    check_id(
      catalog,
      "rice.desktop",
      id,
      &[Category::Desktop],
      &mut issues,
    );
  }
  if let Some(id) = &rice.compositor {
    check_id(
      catalog,
      "rice.compositor",
      id,
      &[Category::Compositor],
      &mut issues,
    );
  }
  if let Some(id) = &rice.shell {
    check_id(catalog, "rice.shell", id, &[Category::Shell], &mut issues);
  }
  for (slot, id) in rice.components.all() {
    if let Some(id) = id {
      // A desktop shell (like Noctalia) can provide components itself,
      // so both categories are allowed here.
      check_id(
        catalog,
        &format!("rice.components.{slot}"),
        id,
        &[Category::Component, Category::Shell],
        &mut issues,
      );
    }
  }

  check_capabilities(profile, catalog, &mut issues);
  check_protocols(profile, catalog, &mut issues);

  issues
}

/// Each slot expects an entry that can actually do that job, e.g. the
/// launcher slot needs something with the `launcher` capability.
fn check_capabilities(profile: &Profile, catalog: &Catalog, issues: &mut Vec<Issue>) {
  let components = &profile.rice.components;
  let slots = [
    (
      "applications.terminal",
      &profile.applications.terminal,
      Capability::Terminal,
    ),
    (
      "rice.components.taskbar",
      &components.taskbar,
      Capability::Panel,
    ),
    (
      "rice.components.status",
      &components.status,
      Capability::Panel,
    ),
    (
      "rice.components.launcher",
      &components.launcher,
      Capability::Launcher,
    ),
    (
      "rice.components.notifications",
      &components.notifications,
      Capability::Notifications,
    ),
  ];

  for (location, id, capability) in slots {
    let Some(entry) = id.as_deref().and_then(|id| catalog.get(id)) else {
      continue; // missing IDs are already reported
    };

    if !entry.capabilities.contains(&capability) {
      issues.push(Issue::warning(
        location,
        format!(
          "'{}' does not list the '{}' capability, so it may not fit this slot",
          entry.id,
          capability_id(capability)
        ),
      ));
    }
  }
}

/// A Wayland-only bar will not run under an X11-only window manager, and
/// the other way round.
fn check_protocols(profile: &Profile, catalog: &Catalog, issues: &mut Vec<Issue>) {
  let rice = &profile.rice;

  // The session is decided by the compositor, or the desktop if there is none.
  let Some(session) = rice
    .compositor
    .as_deref()
    .or(rice.desktop.as_deref())
    .and_then(|id| catalog.get(id))
  else {
    return;
  };
  if session.protocols.is_empty() {
    return;
  }

  let mut parts = vec![("rice.shell", &rice.shell)];
  parts.extend(rice.components.all());

  for (field, id) in parts {
    let Some(entry) = id.as_deref().and_then(|id| catalog.get(id)) else {
      continue;
    };
    if entry.protocols.is_empty() {
      continue;
    }

    let fits = entry
      .protocols
      .iter()
      .any(|p| session.protocols.contains(p));
    if !fits {
      let location = if field.starts_with("rice.") {
        field.to_string()
      } else {
        format!("rice.components.{field}")
      };
      issues.push(Issue::warning(
        location,
        format!(
          "'{}' only works on {}, but '{}' runs on {}",
          entry.id,
          protocols(&entry.protocols),
          session.id,
          protocols(&session.protocols)
        ),
      ));
    }
  }
}

fn protocols(list: &[Protocol]) -> String {
  list
    .iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>()
    .join(" and ")
}

fn capability_id(capability: Capability) -> String {
  serde_json::to_value(capability)
    .ok()
    .and_then(|value| value.as_str().map(str::to_string))
    .unwrap_or_default()
}

/// Checks that `id` exists in the catalog and has one of the `allowed`
/// categories.
fn check_id(
  catalog: &Catalog,
  location: &str,
  id: &str,
  allowed: &[Category],
  issues: &mut Vec<Issue>,
) {
  if id.trim().is_empty() {
    issues.push(Issue::error(location, "the value is empty"));
    return;
  }

  let Some(entry) = catalog.get(id) else {
    let mut message = format!("'{id}' is not in the catalog");
    let similar = catalog.similar_ids(id);
    if !similar.is_empty() {
      message.push_str(&format!(" (did you mean '{}'?)", similar.join("', '")));
    }
    issues.push(Issue::error(location, message));
    return;
  };

  if !allowed.contains(&entry.category) {
    let expected: Vec<&str> = allowed.iter().map(|category| category.id()).collect();
    issues.push(Issue::error(
      location,
      format!(
        "'{id}' is a {}, but this field needs a {}",
        entry.category.id(),
        expected.join(" or ")
      ),
    ));
  }
}

/// Checks the profile's custom `scripts`: each must be a file inside the
/// profile folder. `profile_dir` is the folder that holds `profile.yml`.
pub fn validate_scripts(profile: &Profile, profile_dir: &std::path::Path) -> Vec<Issue> {
  use std::path::{Component, Path};

  let mut issues = Vec::new();
  let mut seen = HashSet::new();

  for (index, script) in profile.scripts.iter().enumerate() {
    let location = format!("scripts[{index}]");
    let path = Path::new(script);

    if script.trim().is_empty() {
      issues.push(Issue::error(location, "the script path is empty"));
      continue;
    }
    if path.is_absolute() || script.starts_with('~') {
      issues.push(Issue::error(
        location,
        format!("'{script}' must be relative to the profile folder, e.g. scripts/tweaks.sh"),
      ));
      continue;
    }
    if path.components().any(|c| c == Component::ParentDir) {
      issues.push(Issue::error(
        location,
        format!("'{script}' points outside the profile folder; scripts must live inside it"),
      ));
      continue;
    }
    if !seen.insert(script.clone()) {
      issues.push(Issue::warning(
        location,
        format!("'{script}' is listed twice"),
      ));
      continue;
    }

    let full = profile_dir.join(path);
    if !full.is_file() {
      issues.push(Issue::error(
        location,
        format!(
          "the script '{script}' does not exist (expected {})",
          full.display()
        ),
      ));
    }
  }

  issues
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::profile::loader::parse;
  use crate::validation::{Severity, count};
  use std::path::Path;

  fn catalog() -> Catalog {
    Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap()
  }

  fn profile(extra: &str) -> Profile {
    let yaml = format!(
      "format_version: 1\nname: T\ndevice:\n  type: desktop\n  os:\n    family: arch\n    distribution: arch\n{extra}"
    );
    parse(&yaml, "t.yml").unwrap()
  }

  #[test]
  fn valid_profile_has_no_issues() {
    let profile = profile(
      "applications:\n  terminal: kitty\nrice:\n  compositor: hyprland\n  shell: noctalia\n",
    );
    let issues = validate(&profile, &catalog());
    assert!(issues.is_empty(), "{issues:#?}");
  }

  #[test]
  fn unknown_id_is_an_error_with_suggestion() {
    let profile = profile("rice:\n  compositor: hyprlnd\n");
    let issues = validate(&profile, &catalog());
    assert_eq!(count(&issues, Severity::Error), 1);
    assert_eq!(issues[0].location, "rice.compositor");
    assert!(issues[0].message.contains("did you mean 'hyprland'"));
  }

  #[test]
  fn wrong_category_is_an_error() {
    let profile = profile("applications:\n  terminal: niri\n");
    let issues = validate(&profile, &catalog());
    assert!(issues[0].message.contains("is a compositor"), "{issues:#?}");
  }

  #[test]
  fn duplicates_are_warnings() {
    let profile = profile("applications:\n  utilities: [kitty]\n  system: [kitty]\n");
    let issues = validate(&profile, &catalog());
    assert_eq!(count(&issues, Severity::Error), 0);
    assert_eq!(count(&issues, Severity::Warning), 1);
  }

  #[test]
  fn wrong_capability_is_a_warning() {
    let profile = profile("rice:\n  components:\n    launcher: dunst\n");
    let issues = validate(&profile, &catalog());
    assert_eq!(count(&issues, Severity::Error), 0);
    assert_eq!(count(&issues, Severity::Warning), 1, "{issues:#?}");
    assert!(issues[0].message.contains("'launcher'"));
  }

  #[test]
  fn repo_test_profile_is_valid() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles/test/profile.yml");
    let profile = crate::profile::loader::load(&path).unwrap();
    let issues = validate(&profile, &catalog());
    assert!(issues.is_empty(), "{issues:#?}");
  }

  #[test]
  fn shell_can_fill_a_component_slot() {
    let profile = profile("rice:\n  components:\n    notifications: noctalia\n");
    assert!(validate(&profile, &catalog()).is_empty());
  }
}
