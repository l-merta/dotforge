//! Detecting the host system.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::backend::{ArchBackend, PackageBackend, package_manager_for};
use super::os_release::{self, OsRelease};
use super::pacman;
use crate::profile::model::OsFamily;

pub struct Host {
  /// Normally `/`. Tests point this at a fake system folder.
  pub root: PathBuf,
  pub os: Option<OsRelease>,
  pub family: Option<OsFamily>,
  /// `None` when the family is not supported or its package database
  /// could not be read.
  pub backend: Option<Box<dyn PackageBackend>>,
  /// Enabled package repositories (Arch: from pacman.conf).
  pub repositories: Option<Vec<String>>,
  /// `wayland` or `x11` for the current graphical session, if any.
  pub session_type: Option<String>,
  pub systemd: bool,
}

/// The folder treated as `/`. `$DOTFORGE_SYSTEM_ROOT` overrides it, which
/// the tests use to fake a whole system.
pub fn system_root() -> PathBuf {
  std::env::var_os("DOTFORGE_SYSTEM_ROOT")
    .filter(|value| !value.is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("/"))
}

impl Host {
  pub fn detect() -> Self {
    Self::detect_at(&system_root())
  }

  pub fn detect_at(root: &Path) -> Self {
    let os = os_release::read(root);
    let family = os.as_ref().and_then(os_release::family_of);

    let backend: Option<Box<dyn PackageBackend>> = match family {
      Some(OsFamily::Arch) => pacman::installed(root).map(|installed| {
        Box::new(ArchBackend {
          installed,
          aur_helper: pacman::find_aur_helper(),
          repositories: pacman::enabled_repositories(root),
        }) as Box<dyn PackageBackend>
      }),
      None => None,
    };

    let repositories = match family {
      Some(OsFamily::Arch) => {
        pacman::enabled_repositories(root).map(|repos| repos.into_iter().collect())
      }
      None => None,
    };

    Self {
      root: root.to_path_buf(),
      os,
      family,
      backend,
      repositories,
      session_type: std::env::var("XDG_SESSION_TYPE")
        .ok()
        .filter(|value| !value.is_empty()),
      systemd: root.join("run/systemd/system").is_dir(),
    }
  }

  /// A readable name for the system, e.g. `EndeavourOS`.
  pub fn name(&self) -> String {
    self
      .os
      .as_ref()
      .map(|os| os.name.clone())
      .unwrap_or_else(|| "an unknown system".into())
  }

  /// Why packages cannot be checked on this host, if they can't.
  pub fn unsupported_reason(&self) -> Option<String> {
    if self.backend.is_some() {
      return None;
    }

    Some(match self.family {
      None => format!(
        "DotForge can't check packages on {} yet; only the Arch family (Arch, EndeavourOS, Manjaro, ...) is supported",
        self.name()
      ),
      Some(family) => format!(
        "could not read the {} package database at {}",
        package_manager_for(family),
        self.root.join("var/lib/pacman/local").display()
      ),
    })
  }

  pub fn summary(&self) -> HostSummary {
    HostSummary {
      root: self.root.display().to_string(),
      distribution: self.os.as_ref().map(|os| os.id.clone()),
      name: self.os.as_ref().map(|os| os.name.clone()),
      version: self.os.as_ref().and_then(|os| os.version_id.clone()),
      based_on: self
        .os
        .as_ref()
        .map(|os| os.id_like.clone())
        .unwrap_or_default(),
      family: self.family,
      supported: self.backend.is_some(),
      package_manager: self
        .backend
        .as_ref()
        .map(|b| b.package_manager().to_string()),
      installed_packages: self.backend.as_ref().map(|b| b.installed_count()),
      aur_helper: self
        .backend
        .as_ref()
        .and_then(|b| b.aur_helper().map(str::to_string)),
      repositories: self.repositories.clone(),
      session_type: self.session_type.clone(),
      systemd: self.systemd,
      problem: self.unsupported_reason(),
    }
  }
}

/// What `dotforge system show` prints.
#[derive(Debug, Clone, Serialize)]
pub struct HostSummary {
  pub root: String,
  pub distribution: Option<String>,
  pub name: Option<String>,
  pub version: Option<String>,
  pub based_on: Vec<String>,
  pub family: Option<OsFamily>,
  /// Can DotForge check (and later install) packages here?
  pub supported: bool,
  pub package_manager: Option<String>,
  pub installed_packages: Option<usize>,
  pub aur_helper: Option<String>,
  pub repositories: Option<Vec<String>>,
  pub session_type: Option<String>,
  pub systemd: bool,
  pub problem: Option<String>,
}
