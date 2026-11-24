use clap::{Parser, Subcommand};

pub mod catalog;
pub mod config;
pub mod profile;

#[derive(Parser, Debug)]
#[command(name = "dotforge")]
#[command(version)]
#[command(about = "DotForge - A Linux desktop setup and rice manager")]
pub struct Cli {
  /// Print the result as JSON instead of text (works with every command)
  #[arg(long, global = true)]
  pub json: bool,

  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
  /// Browse and check the DotForge catalog
  Catalog {
    #[command(subcommand)]
    command: catalog::CatalogCommands,
  },

  /// Manage DotForge profiles
  Profile {
    #[command(subcommand)]
    command: profile::ProfileCommands,
  },

  /// Show or change DotForge's own settings
  Config {
    #[command(subcommand)]
    command: config::ConfigCommands,
  },
}
