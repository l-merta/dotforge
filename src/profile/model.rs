use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
  pub name: String,
  pub device: Device,
  pub desktop: Desktop,
  pub applications: Vec<String>,
  pub theme: Theme,
}

#[derive(Debug, Deserialize)]
pub struct Device {
  pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct Desktop {
  pub compositor: String,
  pub shell: String,
}

#[derive(Debug, Deserialize)]
pub struct Theme {
  pub active: String,
}