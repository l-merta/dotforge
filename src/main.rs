use std::path::Path;

use clap::Parser;

use cli::{Cli, Commands};

mod catalog;
mod cli;

fn main() {
  let cli = Cli::parse();
  let catalog_path = Path::new("catalog");

  match cli.command {
    Commands::Catalog { command } => {
      cli::catalog::handle(command, catalog_path);
    }

    Commands::Profile { command } => {
      cli::profile::handle(command);
    }
  }
}