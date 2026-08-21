use clap::{Parser, Subcommand};

pub mod catalog;
pub mod profile;

#[derive(Parser, Debug)]
#[command(name = "dotforge")]
#[command(version)]
#[command(about = "Dotforge - A Linux desktop setup and rice manager")]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
  /// Manage the Dotforge catalog
  Catalog {
    #[command(subcommand)]
    command: catalog::CatalogCommands,
  },

  /// Manage Dotforge profiles
  Profile {
    #[command(subcommand)]
    command: profile::ProfileCommands,
  },
}