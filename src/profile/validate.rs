//! Checks a parsed profile against the catalog.
//!
//! Parsing (in `loader.rs`) already guarantees that required fields exist and
//! that fixed values such as `device.type` are valid. This file checks the
//! things YAML cannot: that every ID points to a real catalog entry of the
//! right kind.

use std::collections::HashSet;

use super::model::Profile;
use crate::catalog::catalog::Catalog;
use crate::catalog::model::Category;
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

  issues
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
  fn shell_can_fill_a_component_slot() {
    let profile = profile("rice:\n  components:\n    notifications: noctalia\n");
    assert!(validate(&profile, &catalog()).is_empty());
  }
}
