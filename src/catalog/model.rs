//! The data structures that describe one catalog entry (one YAML file).
//! See `docs/catalog-format.md` for the documented format.

use std::collections::BTreeMap;
use std::fmt;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
  pub id: String,
  pub name: String,
  pub description: String,
  pub category: Category,

  #[serde(default)]
  pub platforms: Vec<Platform>,

  #[serde(default)]
  pub protocols: Vec<Protocol>,

  pub installation: Installation,

  /// IDs of other catalog entries this entry needs.
  #[serde(default)]
  pub dependencies: Vec<String>,

  #[serde(default)]
  pub capabilities: Vec<Capability>,

  #[serde(default)]
  pub configuration: Option<Configuration>,

  #[serde(default)]
  pub integration: Option<Integration>,

  /// Steps needed after the packages are installed, such as enabling a
  /// service. Only the typed actions in [`SetupAction`] are allowed.
  #[serde(default)]
  pub setup: Vec<SetupAction>,

  /// Where this entry was loaded from. Not part of the YAML file.
  #[serde(skip_deserializing, default)]
  pub source: Source,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Category {
  Desktop,
  Compositor,
  Shell,
  Component,
  Application,
}

impl Category {
  pub const ALL: [Category; 5] = [
    Category::Desktop,
    Category::Compositor,
    Category::Shell,
    Category::Component,
    Category::Application,
  ];

  /// The folder entries of this category live in, e.g. `compositors/`.
  pub fn folder_name(self) -> &'static str {
    match self {
      Category::Desktop => "desktops",
      Category::Compositor => "compositors",
      Category::Shell => "shells",
      Category::Component => "components",
      Category::Application => "applications",
    }
  }

  /// The value written in YAML, e.g. `compositor`.
  pub fn id(self) -> &'static str {
    match self {
      Category::Desktop => "desktop",
      Category::Compositor => "compositor",
      Category::Shell => "shell",
      Category::Component => "component",
      Category::Application => "application",
    }
  }

  /// A plural, readable title, e.g. `Compositors`.
  pub fn title(self) -> &'static str {
    match self {
      Category::Desktop => "Desktop environments",
      Category::Compositor => "Compositors",
      Category::Shell => "Shells",
      Category::Component => "Components",
      Category::Application => "Applications",
    }
  }
}

impl fmt::Display for Category {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Category::Desktop => "Desktop environment",
      Category::Compositor => "Compositor",
      Category::Shell => "Shell",
      Category::Component => "Component",
      Category::Application => "Application",
    };

    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
  Linux,
}

impl fmt::Display for Platform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Platform::Linux => "Linux",
    };

    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
  Wayland,
  X11,
}

impl fmt::Display for Protocol {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Protocol::Wayland => "Wayland",
      Protocol::X11 => "X11",
    };

    write!(f, "{value}")
  }
}

/// Package names per OS family, for example:
///
/// ```yaml
/// installation:
///   arch:
///     packages: [hyprland]          # official repositories (pacman)
///     aur: [heroic-games-launcher-bin]  # the AUR (paru or yay)
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Installation {
  #[serde(flatten)]
  pub package_managers: BTreeMap<String, PackageInstallation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageInstallation {
  /// Packages from the distribution's official repositories.
  #[serde(default)]
  pub packages: Vec<String>,

  /// Packages from the Arch User Repository, installed through an AUR
  /// helper (paru or yay). Only valid for the `arch` family.
  #[serde(default)]
  pub aur: Vec<String>,

  /// Package repositories that must be enabled for `packages`, e.g.
  /// `multilib` for Steam. Arch only.
  #[serde(default)]
  pub repositories: Vec<String>,
}

impl PackageInstallation {
  pub fn is_empty(&self) -> bool {
    self.packages.is_empty() && self.aur.is_empty()
  }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
  Compositor,
  WindowManagement,
  Keybindings,
  Workspaces,
  Animations,
  Notifications,
  Launcher,
  Panel,
  Wallpaper,
  Theme,
  Terminal,
  DisplayManager,
  Portal,
  Lockscreen,
}

impl fmt::Display for Capability {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Capability::Compositor => "Compositor",
      Capability::WindowManagement => "Window management",
      Capability::Keybindings => "Keybindings",
      Capability::Workspaces => "Workspaces",
      Capability::Animations => "Animations",
      Capability::Notifications => "Notifications",
      Capability::Launcher => "Launcher",
      Capability::Panel => "Panel",
      Capability::Wallpaper => "Wallpaper",
      Capability::Theme => "Theme",
      Capability::Terminal => "Terminal",
      Capability::DisplayManager => "Display manager",
      Capability::Portal => "Desktop portal",
      Capability::Lockscreen => "Lock screen",
    };

    write!(f, "{value}")
  }
}

/// Where an entry's configuration lives on the machine.
///
/// The matching files in a profile are `config/<entry id>/`. From
/// Milestone 6, DotForge links `directory` to that folder with a symlink,
/// so the profile stays the one place to edit them.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
  /// Destination folder, e.g. `~/.config/hypr`. Must start with `~/` or `/`.
  pub directory: String,
}

/// A typed step that installing a package alone does not do.
///
/// Written in YAML like this:
///
/// ```yaml
/// setup:
///   - action: enable_service
///     service: docker.service
///   - action: add_user_to_group
///     group: docker
///   - action: set_default_shell
///     shell: /usr/bin/zsh
/// ```
///
/// These are descriptions, not shell commands: DotForge decides how to
/// run each one and can check whether it is already done (Milestone 5).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SetupAction {
  /// `systemctl enable <service>` (with `--user` when `user: true`).
  EnableService {
    service: String,
    #[serde(default)]
    user: bool,
  },
  /// Add the current user to a group, e.g. `docker` or `input`.
  AddUserToGroup { group: String },
  /// Make this program the user's login shell.
  SetDefaultShell { shell: String },
}

impl fmt::Display for SetupAction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SetupAction::EnableService {
        service,
        user: false,
      } => {
        write!(f, "enable the system service {service}")
      }
      SetupAction::EnableService {
        service,
        user: true,
      } => {
        write!(f, "enable the user service {service}")
      }
      SetupAction::AddUserToGroup { group } => write!(f, "add you to the '{group}' group"),
      SetupAction::SetDefaultShell { shell } => write!(f, "make {shell} your login shell"),
    }
  }
}

/// Which parts of a rice this entry can take part in.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Integration {
  #[serde(default)]
  pub themes: bool,
  #[serde(default)]
  pub wallpapers: bool,
  #[serde(default)]
  pub keybindings: bool,
}

/// Where a catalog entry came from.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Source {
  pub kind: SourceKind,
  /// For built-in entries: the path inside the `catalog/` folder.
  /// For user entries: the full path of the file.
  pub file: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
  /// Built into the program.
  #[default]
  Builtin,
  /// From the user's catalog folder (`~/.config/dotforge/catalog/`).
  User,
}

impl fmt::Display for Source {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.kind {
      SourceKind::Builtin => write!(f, "built-in ({})", self.file),
      SourceKind::User => write!(f, "user ({})", self.file),
    }
  }
}
