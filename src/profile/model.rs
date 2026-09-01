use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
  pub name: String,
  pub description: String,
  pub version: String,
  pub device: Device,
  pub system: System,
  pub applications: Applications,
  pub rice: Rice,
}

#[derive(Debug, Deserialize)]
pub struct Device {
  pub r#type: String,
  pub os: Os,
}

#[derive(Debug, Deserialize)]
pub struct Os {
  pub family: String,
  pub distribution: String,
}

#[derive(Debug, Deserialize)]
pub struct System {
  pub audio: Audio,
  pub graphics: Graphics,
  pub hardware: Hardware,
}

#[derive(Debug, Deserialize)]
pub struct Audio {
  pub backend: String,
}

#[derive(Debug, Deserialize)]
pub struct Graphics {
  pub drivers: Vec<String>,
  pub api: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Hardware {
  pub drivers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Applications {
  pub terminal: String,
  pub gaming: Vec<String>,
  pub social: Vec<String>,
  pub productivity: Vec<String>,
  pub utilities: Vec<String>,
  pub development: Vec<String>,
  pub system: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Rice {
  pub desktop: String,
  pub compositor: String,
  pub shell: String,
  pub components: Components,
}

#[derive(Debug, Deserialize)]
pub struct Components {
  pub taskbar: String,
  pub status: String,
  pub launcher: String,
  pub notifications: String,
}