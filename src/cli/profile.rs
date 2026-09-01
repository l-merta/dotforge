use std::path::Path;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
  /// List available profiles
  List,

  /// Create a new profile
  Create {
    /// Profile name
    name: String,
  },

  /// Apply a profile
  Apply {
    /// Profile name
    name: String,
  },
}

pub fn handle(command: ProfileCommands, profiles_path: &Path) {
  match command {
    ProfileCommands::List => {
      println!("Profile list");
      //list(profiles_path);
    }

    ProfileCommands::Create { name } => {
      println!("Creating profile: {name}");
    }

    ProfileCommands::Apply { name } => {
      println!("Applying profile: {name}");
    }
  }
}