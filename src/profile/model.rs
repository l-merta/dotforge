//! The data structures of `profile.yml`.
//! Every field is documented in `docs/profile-format.md`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The profile format version this build of DotForge understands.
/// Bump this only when an old `profile.yml` would no longer be read correctly.
pub const CURRENT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
  /// Which version of the profile *format* this file uses (not a version
  /// of your setup). Currently always `1`.
  pub format_version: u32,

  pub name: String,

  #[serde(default)]
  pub description: String,

  pub device: Device,

  #[serde(default)]
  pub system: System,

  #[serde(default)]
  pub applications: Applications,

  #[serde(default)]
  pub rice: Rice,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
  #[serde(rename = "type")]
  pub kind: DeviceType,
  pub os: Os,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceType {
  Desktop,
  Laptop,
  VirtualMachine,
}

impl fmt::Display for DeviceType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      DeviceType::Desktop => "desktop",
      DeviceType::Laptop => "laptop",
      DeviceType::VirtualMachine => "virtual-machine",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Os {
  /// The group of distributions that share a package manager.
  pub family: OsFamily,
  /// The specific distribution, e.g. `arch` or `endeavouros`.
  pub distribution: String,
}

/// Only the Arch family is supported for now. More families
/// (Debian, Fedora, ...) are added when they get a package-manager backend.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OsFamily {
  Arch,
}

impl fmt::Display for OsFamily {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OsFamily::Arch => write!(f, "arch"),
    }
  }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct System {
  #[serde(default)]
  pub audio: Option<Audio>,
  #[serde(default)]
  pub graphics: Graphics,
  #[serde(default)]
  pub hardware: Hardware,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audio {
  pub backend: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Graphics {
  #[serde(default)]
  pub drivers: Vec<String>,
  #[serde(default)]
  pub api: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hardware {
  #[serde(default)]
  pub drivers: Vec<String>,
}

/// Every value is a catalog ID of category `application`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Applications {
  #[serde(default)]
  pub terminal: Option<String>,
  #[serde(default)]
  pub gaming: Vec<String>,
  #[serde(default)]
  pub social: Vec<String>,
  #[serde(default)]
  pub productivity: Vec<String>,
  #[serde(default)]
  pub utilities: Vec<String>,
  #[serde(default)]
  pub development: Vec<String>,
  #[serde(default)]
  pub system: Vec<String>,
}

impl Applications {
  /// All application lists with their field names, in file order.
  pub fn groups(&self) -> [(&'static str, &[String]); 6] {
    [
      ("gaming", &self.gaming),
      ("social", &self.social),
      ("productivity", &self.productivity),
      ("utilities", &self.utilities),
      ("development", &self.development),
      ("system", &self.system),
    ]
  }
}

/// The look and feel of the desktop. Every value is a catalog ID.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rice {
  #[serde(default)]
  pub desktop: Option<String>,
  #[serde(default)]
  pub compositor: Option<String>,
  #[serde(default)]
  pub shell: Option<String>,
  #[serde(default)]
  pub components: Components,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Components {
  #[serde(default)]
  pub taskbar: Option<String>,
  #[serde(default)]
  pub status: Option<String>,
  #[serde(default)]
  pub launcher: Option<String>,
  #[serde(default)]
  pub notifications: Option<String>,
}

impl Components {
  pub fn all(&self) -> [(&'static str, &Option<String>); 4] {
    [
      ("taskbar", &self.taskbar),
      ("status", &self.status),
      ("launcher", &self.launcher),
      ("notifications", &self.notifications),
    ]
  }
}
