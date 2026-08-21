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

pub fn handle(command: ProfileCommands) {
  match command {
    ProfileCommands::List => {
      println!("Profile list");
    }

    ProfileCommands::Create { name } => {
      println!("Creating profile: {name}");
    }

    ProfileCommands::Apply { name } => {
      println!("Applying profile: {name}");
    }
  }
}