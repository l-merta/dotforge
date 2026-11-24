use clap::Parser;

use cli::{Cli, Commands};

mod apply;
mod catalog;
mod cli;
mod editor;
mod error;
mod exit_code;
mod output;
mod paths;
mod profile;
mod settings;
mod system;
mod validation;

fn main() -> std::process::ExitCode {
  let cli = Cli::parse();

  // 1. Run the command. It returns data, it does not print anything.
  let result = match cli.command {
    Commands::Catalog { command } => cli::catalog::handle(command),
    Commands::Profile { command } => cli::profile::handle(command),
    Commands::Config { command } => cli::config::handle(command),
    Commands::System { command } => cli::system::handle(command),
  };

  // 2. Print the result as text or JSON and exit with the matching code.
  let exit_code = output::print_result(result, cli.json);
  std::process::ExitCode::from(exit_code.code())
}
