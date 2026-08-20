use std::fs;
use std::path::Path;

use super::model::Profile;

pub fn load_profile(path: &Path) -> Result<Profile, Box<dyn std::error::Error>> {
  let contents = fs::read_to_string(path)?;

  let profile: Profile = serde_yaml::from_str(&contents)?;

  Ok(profile)
}