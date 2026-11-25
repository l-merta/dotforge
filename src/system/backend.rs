//! Package-manager backends, one per OS family.
//!
//! A backend knows which key in a catalog entry's `installation:` section
//! belongs to it (`arch`), whether a package is installed, and which
//! commands install or remove packages. Keeping this behind a trait means
//! profiles and catalog entries never contain pacman/apt/dnf details, and
//! a new family only needs a new backend.

use super::pacman::InstalledPackages;
use crate::apply::command::CommandSpec;
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

  /// The command that installs official packages. Packages that are
  /// already installed are skipped by the package manager itself.
  fn install_command(&self, packages: &[String]) -> CommandSpec;
  /// The command that installs AUR packages, or `None` without a helper.
  fn aur_install_command(&self, packages: &[String]) -> Option<CommandSpec>;
  /// The command that removes packages (and dependencies nothing else needs).
  fn remove_command(&self, packages: &[String]) -> CommandSpec;
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

  fn install_command(&self, packages: &[String]) -> CommandSpec {
    // DotForge has already asked for confirmation, so pacman doesn't need
    // to ask again. `--needed` skips anything that is already installed.
    CommandSpec::sudo("pacman", ["-S", "--needed", "--noconfirm"], packages)
  }

  fn aur_install_command(&self, packages: &[String]) -> Option<CommandSpec> {
    // AUR helpers must run as your own user, never with sudo; they ask for
    // sudo themselves when they install the built package.
    self
      .aur_helper
      .as_deref()
      .map(|helper| CommandSpec::user(helper, ["-S", "--needed", "--noconfirm"], packages))
  }

  fn remove_command(&self, packages: &[String]) -> CommandSpec {
    CommandSpec::sudo("pacman", ["-Rs", "--noconfirm"], packages)
  }
}

/// The package manager each supported family uses (for display before a
/// backend is built).
pub fn package_manager_for(family: OsFamily) -> &'static str {
  match family {
    OsFamily::Arch => "pacman",
  }
}
