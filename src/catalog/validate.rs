//! Checks the whole catalog against the rules in `docs/catalog-format.md`.

use std::collections::HashSet;
use std::path::Path;

use super::catalog::Catalog;
use super::conventions::{is_valid_group, is_valid_id, is_valid_package};
use super::model::{Capability, CatalogEntry, Category, SetupAction, SourceKind};
use super::resolve::find_cycles;
use crate::validation::Issue;

/// The OS families DotForge knows. Only Arch is supported for now.
pub const KNOWN_FAMILIES: [&str; 1] = ["arch"];

pub fn validate(catalog: &Catalog) -> Vec<Issue> {
  let mut issues = Vec::new();

  for entry in catalog.all() {
    let mut report = Report {
      entry,
      issues: &mut issues,
    };
    check_naming(&mut report);
    check_category_rules(&mut report);
    check_installation(&mut report);
    check_dependencies(&mut report, catalog);
    check_configuration(&mut report);
    check_setup(&mut report);
  }

  for cycle in find_cycles(catalog) {
    issues.push(Issue::error(
      &cycle[0],
      format!("circular dependency: {}", cycle.join(" → ")),
    ));
  }

  issues
}

/// Small helper so each check can add issues for "this entry".
struct Report<'a> {
  entry: &'a CatalogEntry,
  issues: &'a mut Vec<Issue>,
}

impl Report<'_> {
  fn error(&mut self, field: Option<&str>, message: impl Into<String>) {
    let location = self.location(field);
    self.issues.push(Issue::error(location, message));
  }

  fn warning(&mut self, field: Option<&str>, message: impl Into<String>) {
    let location = self.location(field);
    self.issues.push(Issue::warning(location, message));
  }

  /// Built-in entries must follow every rule; for your own entries some
  /// rules are only tips.
  fn strict(&mut self, field: Option<&str>, message: impl Into<String>) {
    match self.entry.source.kind {
      SourceKind::Builtin => self.error(field, message),
      SourceKind::User => self.warning(field, message),
    }
  }

  fn location(&self, field: Option<&str>) -> String {
    match field {
      Some(field) => format!("{}.{field}", self.entry.id),
      None => self.entry.id.clone(),
    }
  }
}

fn check_naming(r: &mut Report) {
  let entry = r.entry;
  let path = Path::new(&entry.source.file);

  if !is_valid_id(&entry.id) {
    r.error(
      None,
      "IDs may only use lowercase letters, digits and single dashes (for example 'kde-plasma')",
    );
  }

  if entry.name.trim().is_empty() {
    r.error(Some("name"), "'name' is empty");
  }

  if entry.description.trim().is_empty() {
    r.warning(Some("description"), "'description' is empty");
  }

  let stem = path
    .file_stem()
    .and_then(|stem| stem.to_str())
    .unwrap_or("");
  if stem != entry.id {
    r.error(
      None,
      format!(
        "file '{}' should be named '{}.yml' to match its ID",
        entry.source.file, entry.id
      ),
    );
  }

  let folder = path
    .parent()
    .and_then(|parent| parent.file_name())
    .and_then(|name| name.to_str())
    .unwrap_or("");
  let expected = entry.category.folder_name();
  if folder != expected {
    r.strict(
      None,
      format!(
        "category is '{}', so the file belongs in a '{expected}/' folder",
        entry.category.id()
      ),
    );
  }
}

/// Rules that depend on the kind of entry.
fn check_category_rules(r: &mut Report) {
  let entry = r.entry;
  let has = |capability: Capability| entry.capabilities.contains(&capability);

  match entry.category {
    Category::Desktop | Category::Compositor | Category::Shell => {
      if entry.protocols.is_empty() {
        r.error(
          Some("protocols"),
          format!(
            "a {} must list the display protocols it works with (wayland, x11)",
            entry.category.id()
          ),
        );
      }
    }
    Category::Component | Category::Application => {}
  }

  if entry.category == Category::Compositor && !has(Capability::Compositor) {
    r.warning(
      Some("capabilities"),
      "a compositor should list the 'compositor' capability",
    );
  }

  if entry.category == Category::Component && entry.capabilities.is_empty() {
    r.warning(
      Some("capabilities"),
      "a component should list what it provides (e.g. panel, launcher, notifications)",
    );
  }

  if entry.platforms.is_empty() {
    r.warning(Some("platforms"), "no platforms listed (add 'linux')");
  }
}

fn check_installation(r: &mut Report) {
  let entry = r.entry;
  let managers = &entry.installation.package_managers;

  if !managers.contains_key("arch") {
    r.strict(
      Some("installation"),
      "no installation for the 'arch' family (Arch is the first supported system)",
    );
  }

  for (family, install) in managers {
    let field = format!("installation.{family}");

    if !KNOWN_FAMILIES.contains(&family.as_str()) {
      r.warning(
        Some(&field),
        format!("'{family}' is not a supported OS family yet (supported: arch)"),
      );
    }

    if install.is_empty() {
      r.error(Some(&field), "lists no packages");
    }

    if !install.aur.is_empty() && family != "arch" {
      r.error(
        Some(&format!("{field}.aur")),
        "AUR packages only exist for the 'arch' family",
      );
    }

    let mut seen = HashSet::new();
    for package in install.packages.iter().chain(&install.aur) {
      if !is_valid_package(package) {
        r.error(
          Some(&field),
          format!("'{package}' is not a valid package name"),
        );
      }
      if !seen.insert(package) {
        r.warning(Some(&field), format!("package '{package}' is listed twice"));
      }
    }
  }
}

fn check_dependencies(r: &mut Report, catalog: &Catalog) {
  let entry = r.entry;
  let mut seen = HashSet::new();

  for (index, dependency) in entry.dependencies.iter().enumerate() {
    let field = format!("dependencies[{index}]");

    if dependency == &entry.id {
      r.error(Some(&field), "an entry cannot depend on itself");
    } else if catalog.get(dependency).is_none() {
      let mut message = format!("'{dependency}' is not in the catalog");
      let similar = catalog.similar_ids(dependency);
      if !similar.is_empty() {
        message.push_str(&format!(" (did you mean '{}'?)", similar.join("', '")));
      }
      r.error(Some(&field), message);
    }

    if !seen.insert(dependency) {
      r.warning(Some(&field), format!("'{dependency}' is listed twice"));
    }
  }
}

fn check_configuration(r: &mut Report) {
  if let Some(configuration) = &r.entry.configuration {
    let directory = configuration.directory.as_str();
    if !(directory.starts_with("~/") || directory.starts_with('/')) {
      r.error(
        Some("configuration.directory"),
        format!("'{directory}' must be a full path starting with '~/' or '/'"),
      );
    }
  }
}

fn check_setup(r: &mut Report) {
  let actions = r.entry.setup.clone();
  let mut seen = Vec::new();

  for (index, action) in actions.iter().enumerate() {
    let field = format!("setup[{index}]");

    match action {
      SetupAction::EnableService { service, .. } => {
        let known_unit = [".service", ".socket", ".timer", ".path"]
          .iter()
          .any(|suffix| service.ends_with(suffix));
        if !known_unit || service.contains('/') || service.contains(' ') {
          r.error(
            Some(&field),
            format!(
              "'{service}' should be a systemd unit name like 'docker.service' or 'bluetooth.service'"
            ),
          );
        }
      }
      SetupAction::AddUserToGroup { group } => {
        if !is_valid_group(group) {
          r.error(Some(&field), format!("'{group}' is not a valid group name"));
        }
      }
      SetupAction::SetDefaultShell { shell } => {
        if !shell.starts_with('/') {
          r.error(
            Some(&field),
            format!("'{shell}' must be the full path of the shell, e.g. '/usr/bin/zsh'"),
          );
        }
      }
    }

    if seen.contains(action) {
      r.warning(Some(&field), "this setup action is listed twice");
    }
    seen.push(action.clone());
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::catalog::loader::parse_entry;
  use crate::catalog::model::Source;
  use crate::validation::{Severity, count};

  fn builtin() -> Catalog {
    Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap()
  }

  /// Validates one extra user entry on top of the built-in catalog and
  /// returns only the issues about it.
  fn check(id: &str, folder: &str, yaml: &str) -> Vec<Issue> {
    let mut entry = parse_entry(yaml, "t.yml").unwrap();
    entry.source = Source {
      kind: SourceKind::User,
      file: format!("/x/{folder}/{id}.yml"),
    };
    let builtin_entries = builtin().all().to_vec();
    let catalog = Catalog::from_entries(builtin_entries, vec![entry]).unwrap();
    validate(&catalog)
      .into_iter()
      .filter(|issue| issue.location.starts_with(id))
      .collect()
  }

  const BASE: &str = "id: demo\nname: Demo\ndescription: d\nplatforms: [linux]\n";

  #[test]
  fn builtin_catalog_is_valid() {
    let issues = validate(&builtin());
    assert!(issues.is_empty(), "{issues:#?}");
  }

  #[test]
  fn good_entry_has_no_issues() {
    let yaml = format!(
      "{BASE}category: application\ninstallation:\n  arch:\n    aur: [demo-bin]\nsetup:\n  - action: enable_service\n    service: demo.service\n  - action: add_user_to_group\n    group: video\n"
    );
    assert!(check("demo", "applications", &yaml).is_empty());
  }

  #[test]
  fn missing_dependency_is_an_error() {
    let yaml = format!(
      "{BASE}category: application\ninstallation:\n  arch:\n    packages: [demo]\ndependencies: [hyprlnd]\n"
    );
    let issues = check("demo", "applications", &yaml);
    assert_eq!(count(&issues, Severity::Error), 1);
    assert!(issues[0].message.contains("did you mean 'hyprland'"));
  }

  #[test]
  fn compositor_needs_protocols() {
    let yaml = format!(
      "{BASE}category: compositor\ncapabilities: [compositor]\ninstallation:\n  arch:\n    packages: [demo]\n"
    );
    let issues = check("demo", "compositors", &yaml);
    assert_eq!(issues[0].location, "demo.protocols");
  }

  #[test]
  fn bad_setup_actions() {
    let yaml = format!(
      "{BASE}category: application\ninstallation:\n  arch:\n    packages: [demo]\nsetup:\n  - action: enable_service\n    service: docker\n  - action: set_default_shell\n    shell: zsh\n  - action: add_user_to_group\n    group: Bad Group\n"
    );
    let issues = check("demo", "applications", &yaml);
    assert_eq!(count(&issues, Severity::Error), 3, "{issues:#?}");
  }

  #[test]
  fn unknown_setup_action_is_a_parse_error() {
    let yaml = format!(
      "{BASE}category: application\ninstallation: {{}}\nsetup:\n  - action: run_command\n    command: rm -rf /\n"
    );
    let error = parse_entry(&yaml, "t.yml").unwrap_err().to_string();
    assert!(error.contains("run_command"), "{error}");
  }

  #[test]
  fn aur_outside_arch_and_relative_config() {
    let yaml = format!(
      "{BASE}category: application\ninstallation:\n  arch:\n    packages: [demo]\n  debian:\n    aur: [demo]\nconfiguration:\n  directory: .config/demo\n"
    );
    let issues = check("demo", "applications", &yaml);
    assert_eq!(count(&issues, Severity::Error), 2, "{issues:#?}");
    assert_eq!(count(&issues, Severity::Warning), 1, "{issues:#?}");
  }
}
