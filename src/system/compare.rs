//! Comparing a profile with the machine: what is already there, what is
//! missing, and what DotForge can't handle here. Read-only.

use serde::Serialize;

use super::host::Host;
use crate::catalog::catalog::Catalog;
use crate::catalog::model::{Category, SetupAction};
use crate::catalog::resolve;
use crate::error::AppError;
use crate::profile::model::Profile;
use crate::validation::Issue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
  /// Every package of this entry is installed.
  Installed,
  /// At least one package needs to be installed.
  Missing,
  /// DotForge can't check or install this entry on this machine.
  Unsupported,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntryState {
  pub id: String,
  pub name: String,
  pub category: Category,
  /// Why this entry is needed: profile fields such as `rice.compositor`,
  /// or `dependency of hyprland`.
  pub wanted_by: Vec<String>,
  pub status: Status,
  /// Official packages that still need installing.
  pub missing_packages: Vec<String>,
  /// AUR packages that still need installing.
  pub missing_aur: Vec<String>,
  /// Extra steps from the catalog. Whether they are done is checked in
  /// Milestone 5, so they are listed but not judged here.
  pub setup: Vec<SetupAction>,
  /// Set when the status is `unsupported`.
  pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
  pub installed: usize,
  pub missing: usize,
  pub unsupported: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comparison {
  pub profile: String,
  pub file: String,
  pub host: String,
  /// `true` when nothing is missing or unsupported.
  pub satisfied: bool,
  pub counts: Counts,
  /// All entries the profile needs, dependencies first.
  pub entries: Vec<EntryState>,
  /// Everything that would be installed, in order.
  pub to_install: ToInstall,
  /// Things worth knowing, e.g. no AUR helper.
  pub notes: Vec<Issue>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ToInstall {
  /// With the package manager (pacman).
  pub packages: Vec<String>,
  /// With the AUR helper (paru or yay).
  pub aur: Vec<String>,
  pub package_manager: Option<String>,
  pub aur_helper: Option<String>,
}

/// The catalog IDs a profile asks for, with the field that asks for each.
pub fn wanted_ids(profile: &Profile) -> Vec<(String, String)> {
  let mut wanted = Vec::new();
  let mut add = |field: String, id: &Option<String>| {
    if let Some(id) = id {
      wanted.push((field, id.clone()));
    }
  };

  let rice = &profile.rice;
  add("rice.desktop".into(), &rice.desktop);
  add("rice.compositor".into(), &rice.compositor);
  add("rice.shell".into(), &rice.shell);
  for (slot, id) in rice.components.all() {
    add(format!("rice.components.{slot}"), id);
  }

  add(
    "applications.terminal".into(),
    &profile.applications.terminal,
  );
  for (group, ids) in profile.applications.groups() {
    for (index, id) in ids.iter().enumerate() {
      wanted.push((format!("applications.{group}[{index}]"), id.clone()));
    }
  }

  wanted
}

pub fn compare(
  profile: &Profile,
  file: &str,
  catalog: &Catalog,
  host: &Host,
) -> Result<Comparison, AppError> {
  let wanted = wanted_ids(profile);
  let roots: Vec<&str> = wanted.iter().map(|(_, id)| id.as_str()).collect();

  let order = resolve::install_order(catalog, &roots).map_err(|error| AppError::Invalid {
    message: error.to_string(),
    hint: Some(format!(
      "run 'dotforge profile validate {file}' to see every problem"
    )),
  })?;

  let mut notes = Vec::new();
  let mut entries = Vec::new();
  let mut counts = Counts::default();
  let mut to_install = ToInstall {
    package_manager: host
      .backend
      .as_ref()
      .map(|b| b.package_manager().to_string()),
    aur_helper: host
      .backend
      .as_ref()
      .and_then(|b| b.aur_helper().map(str::to_string)),
    ..ToInstall::default()
  };

  if let Some(family) = host.family
    && family != profile.device.os.family
  {
    notes.push(Issue::warning(
      "device.os.family",
      format!(
        "the profile is for the '{}' family, but this machine is '{family}'",
        profile.device.os.family
      ),
    ));
  }

  for id in &order {
    let entry = catalog.get(id).expect("resolved IDs exist in the catalog");

    let mut wanted_by: Vec<String> = wanted
      .iter()
      .filter(|(_, wanted_id)| wanted_id == id)
      .map(|(field, _)| field.clone())
      .collect();
    for other in &order {
      if let Some(parent) = catalog.get(other)
        && parent.dependencies.contains(id)
      {
        wanted_by.push(format!("dependency of {other}"));
      }
    }

    let mut state = EntryState {
      id: id.clone(),
      name: entry.name.clone(),
      category: entry.category,
      wanted_by,
      status: Status::Installed,
      missing_packages: Vec::new(),
      missing_aur: Vec::new(),
      setup: entry.setup.clone(),
      reason: None,
    };

    match &host.backend {
      None => {
        state.status = Status::Unsupported;
        state.reason = host.unsupported_reason();
      }
      Some(backend) => match entry
        .installation
        .package_managers
        .get(backend.catalog_key())
      {
        None => {
          state.status = Status::Unsupported;
          state.reason = Some(format!(
            "the catalog has no '{}' packages for {}",
            backend.catalog_key(),
            entry.name
          ));
        }
        Some(install) => {
          state.missing_packages = missing(&install.packages, |p| backend.is_installed(p));
          state.missing_aur = missing(&install.aur, |p| backend.is_installed(p));

          if !state.missing_packages.is_empty() || !state.missing_aur.is_empty() {
            state.status = Status::Missing;
          }

          // Only matters if something still has to be installed from it.
          if !state.missing_packages.is_empty() {
            for repository in &install.repositories {
              if backend.repository_enabled(repository) == Some(false) {
                notes.push(Issue::warning(
                  id.as_str(),
                  format!(
                    "{} needs the '{repository}' repository, which is not enabled in /etc/pacman.conf",
                    entry.name
                  ),
                ));
              }
            }
          }
        }
      },
    }

    match state.status {
      Status::Installed => counts.installed += 1,
      Status::Missing => counts.missing += 1,
      Status::Unsupported => counts.unsupported += 1,
    }
    push_unique(&mut to_install.packages, &state.missing_packages);
    push_unique(&mut to_install.aur, &state.missing_aur);
    entries.push(state);
  }

  if !to_install.aur.is_empty() && to_install.aur_helper.is_none() {
    notes.push(Issue::warning(
      "aur",
      format!(
        "{} AUR package(s) are needed, but neither paru nor yay is installed",
        to_install.aur.len()
      ),
    ));
  }

  Ok(Comparison {
    profile: profile.name.clone(),
    file: file.to_string(),
    host: host.name(),
    satisfied: counts.missing == 0 && counts.unsupported == 0,
    counts,
    entries,
    to_install,
    notes,
  })
}

fn missing(packages: &[String], is_installed: impl Fn(&str) -> bool) -> Vec<String> {
  packages
    .iter()
    .filter(|package| !is_installed(package))
    .cloned()
    .collect()
}

fn push_unique(target: &mut Vec<String>, items: &[String]) {
  for item in items {
    if !target.contains(item) {
      target.push(item.clone());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::profile::loader::parse;
  use std::fs;
  use std::path::Path;

  const PROFILE: &str = "format_version: 1\nname: T\ndevice:\n  type: desktop\n  os:\n    family: arch\n    distribution: arch\napplications:\n  terminal: kitty\n  gaming: [heroic]\nrice:\n  compositor: hyprland\n";

  fn catalog() -> Catalog {
    Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap()
  }

  /// A fake Arch system with some packages installed.
  fn fake_arch(installed: &[&str]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("etc")).unwrap();
    fs::write(
      root.path().join("etc/os-release"),
      "NAME=\"Arch Linux\"\nID=arch\n",
    )
    .unwrap();
    for package in installed {
      let dir = root
        .path()
        .join(format!("var/lib/pacman/local/{package}-1.0-1"));
      fs::create_dir_all(&dir).unwrap();
      fs::write(dir.join("desc"), format!("%NAME%\n{package}\n\n")).unwrap();
    }
    fs::create_dir_all(root.path().join("var/lib/pacman/local")).unwrap();
    fs::write(
      root.path().join("etc/pacman.conf"),
      "[core]\n[extra]\n#[multilib]\n",
    )
    .unwrap();
    root
  }

  #[test]
  fn warns_about_a_disabled_repository() {
    let root = fake_arch(&[]);
    let host = Host::detect_at(root.path());
    let profile = parse(
      &PROFILE.replace("gaming: [heroic]", "gaming: [steam]"),
      "p.yml",
    )
    .unwrap();

    let result = compare(&profile, "p.yml", &catalog(), &host).unwrap();
    assert_eq!(result.notes.len(), 1, "{:#?}", result.notes);
    assert!(result.notes[0].message.contains("'multilib'"));
  }

  #[test]
  fn compares_installed_and_missing() {
    let root = fake_arch(&["kitty", "xdg-desktop-portal"]);
    let host = Host::detect_at(root.path());
    let profile = parse(PROFILE, "p.yml").unwrap();

    let result = compare(&profile, "p.yml", &catalog(), &host).unwrap();

    let ids: Vec<&str> = result.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
      ids,
      [
        "xdg-desktop-portal",
        "xdg-desktop-portal-hyprland",
        "hyprland",
        "kitty",
        "heroic"
      ]
    );

    assert_eq!(result.counts.installed, 2);
    assert_eq!(result.counts.missing, 3);
    assert!(!result.satisfied);
    assert_eq!(
      result.to_install.packages,
      ["xdg-desktop-portal-hyprland", "hyprland"]
    );
    assert_eq!(result.to_install.aur, ["heroic-games-launcher-bin"]);

    let portal = &result.entries[0];
    assert_eq!(
      portal.wanted_by,
      ["dependency of xdg-desktop-portal-hyprland"]
    );
    assert_eq!(result.entries[2].wanted_by, ["rice.compositor"]);
  }

  #[test]
  fn everything_installed_is_satisfied() {
    let root = fake_arch(&[
      "kitty",
      "xdg-desktop-portal",
      "xdg-desktop-portal-hyprland",
      "hyprland",
      "heroic-games-launcher-bin",
    ]);
    let host = Host::detect_at(root.path());
    let profile = parse(PROFILE, "p.yml").unwrap();

    let result = compare(&profile, "p.yml", &catalog(), &host).unwrap();
    assert!(result.satisfied, "{result:#?}");
    assert!(result.to_install.packages.is_empty());
  }

  #[test]
  fn unknown_system_is_unsupported() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("etc")).unwrap();
    fs::write(
      root.path().join("etc/os-release"),
      "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n",
    )
    .unwrap();

    let host = Host::detect_at(root.path());
    let profile = parse(PROFILE, "p.yml").unwrap();
    let result = compare(&profile, "p.yml", &catalog(), &host).unwrap();

    assert_eq!(result.counts.unsupported, result.entries.len());
    assert!(
      result.entries[0]
        .reason
        .as_deref()
        .unwrap()
        .contains("Ubuntu")
    );
  }

  #[test]
  fn wanted_ids_lists_every_field() {
    let profile = parse(PROFILE, "p.yml").unwrap();
    let wanted = wanted_ids(&profile);
    assert_eq!(
      wanted[0],
      ("rice.compositor".to_string(), "hyprland".to_string())
    );
    assert_eq!(wanted.len(), 3);
  }
}
