//! Package-manager backends, one per OS family.
//!
//! A backend knows two things: which key in a catalog entry's
//! `installation:` section belongs to it (`arch`), and whether a package is
//! installed. Installing comes in Milestone 5. Keeping this behind a trait
//! means profiles and catalog entries never contain pacman/apt/dnf details,
//! and a new family only needs a new backend.

use super::pacman::InstalledPackages;
use crate::profile::model::OsFamily;

pub trait PackageBackend {
  /// The key used under `installation:` in catalog entries.
  fn catalog_key(&self) -> &'static str;
  /// The program used for official packages, e.g. `pacman`.
  fn package_manager(&self) -> &'static str;
  /// The program used for AUR packages, if this family has one installed.
  fn aur_helper(&self) -> Option<&str>;
  fn is_installed(&self, package: &str) -> bool;
  /// Is this package repository enabled? `None` if it can't be told.
  fn repository_enabled(&self, repository: &str) -> Option<bool>;
  /// How many packages are installed in total.
  fn installed_count(&self) -> usize;
}

/// The Arch family: pacman for official packages, paru or yay for the AUR.
pub struct ArchBackend {
  pub installed: InstalledPackages,
  pub aur_helper: Option<String>,
  /// From pacman.conf; `None` if it could not be read.
  pub repositories: Option<std::collections::BTreeSet<String>>,
}

impl PackageBackend for ArchBackend {
  fn catalog_key(&self) -> &'static str {
    "arch"
  }

  fn package_manager(&self) -> &'static str {
    "pacman"
  }

  fn aur_helper(&self) -> Option<&str> {
    self.aur_helper.as_deref()
  }

  fn is_installed(&self, package: &str) -> bool {
    self.installed.has(package)
  }

  fn repository_enabled(&self, repository: &str) -> Option<bool> {
    self
      .repositories
      .as_ref()
      .map(|repositories| repositories.contains(repository))
  }

  fn installed_count(&self) -> usize {
    self.installed.count()
  }
}

/// The package manager each supported family uses (for display before a
/// backend is built).
pub fn package_manager_for(family: OsFamily) -> &'static str {
  match family {
    OsFamily::Arch => "pacman",
  }
}
