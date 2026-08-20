use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
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

  #[serde(default)]
  pub dependencies: Vec<String>,

  #[serde(default)]
  pub capabilities: Vec<Capability>,

  #[serde(default)]
  pub configuration: Option<Configuration>,

  #[serde(default)]
  pub integration: Integration,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
  Desktop,
  Compositor,
  Shell,
  Component,
  Application,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
  Linux,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
  Wayland,
  X11,
}

#[derive(Debug, Deserialize)]
pub struct Installation {
  #[serde(flatten)]
  pub package_managers: HashMap<String, PackageInstallation>,
}

#[derive(Debug, Deserialize)]
pub struct PackageInstallation {
  #[serde(default)]
  pub packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Configuration {
  pub directory: String,
}

#[derive(Debug, Deserialize)]
pub struct Integration {
  #[serde(default)]
  pub themes: bool,

  #[serde(default)]
  pub wallpapers: bool,

  #[serde(default)]
  pub keybindings: bool,
}

impl Default for Integration {
  fn default() -> Self {
    Self {
      themes: false,
      wallpapers: false,
      keybindings: false,
    }
  }
}