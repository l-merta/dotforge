use serde::Deserialize;
use clap::ValueEnum;
use std::collections::HashMap;
use std::fmt;

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
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Category {
  Desktop,
  Compositor,
  Shell,
  Component,
  Application,
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Configuration {
  pub directory: String,
}