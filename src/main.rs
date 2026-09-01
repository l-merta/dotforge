use std::path::Path;

use clap::Parser;

use cli::{Cli, Commands};

mod catalog;
mod cli;
mod profile;

fn main() {
  let cli = Cli::parse();
  let catalog_path = Path::new("catalog");
  let profiles_path = Path::new("profiles");

  match cli.command {
    Commands::Catalog { command } => {
      cli::catalog::handle(command, catalog_path);
    }

    Commands::Profile { command } => {
      cli::profile::handle(command, profiles_path);
    }
  }
}