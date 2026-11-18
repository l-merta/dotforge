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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
///     packages: [hyprland]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Installation {
  #[serde(flatten)]
  pub package_managers: BTreeMap<String, PackageInstallation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageInstallation {
  #[serde(default)]
  pub packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    };

    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
  pub directory: String,
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
