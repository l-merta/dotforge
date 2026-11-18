//! `dotforge profile ...` commands.

use clap::Subcommand;
use serde::Serialize;

use crate::catalog::catalog::Catalog;
use crate::error::AppError;
use crate::exit_code::ExitCode;
use crate::output::{Render, Report};
use crate::paths;
use crate::profile::{loader, validate};
use crate::validation::{Issue, Severity, count, render_issues};

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
  /// List available profiles (not implemented yet)
  List,

  /// Create a new profile (not implemented yet)
  Create {
    /// Profile name
    name: String,
  },

  /// Apply a profile (not implemented yet)
  Apply {
    /// Profile name
    name: String,
  },

  /// Check a profile for mistakes
  Validate {
    /// A profile name (looked up in ~/.config/dotforge/profiles/)
    /// or a path to a profile folder or profile.yml
    profile: String,
  },
}

pub fn handle(command: ProfileCommands) -> Result<Report, AppError> {
  match command {
    ProfileCommands::List => Err(AppError::NotImplemented {
      command: "profile list",
      milestone: "Milestone 2",
    }),

    ProfileCommands::Create { .. } => Err(AppError::NotImplemented {
      command: "profile create",
      milestone: "Milestone 2",
    }),

    ProfileCommands::Apply { .. } => Err(AppError::NotImplemented {
      command: "profile apply",
      milestone: "Milestone 5",
    }),

    ProfileCommands::Validate { profile } => validate_profile(&profile),
  }
}

#[derive(Serialize)]
pub struct ProfileValidation {
  pub profile: String,
  pub file: String,
  pub valid: bool,
  pub errors: usize,
  pub warnings: usize,
  pub issues: Vec<Issue>,
}

fn validate_profile(name_or_path: &str) -> Result<Report, AppError> {
  let file = loader::resolve(name_or_path, &paths::profiles_dir()?)?;
  let profile = loader::load(&file)?;
  let catalog = Catalog::load()?;

  let issues = validate::validate(&profile, &catalog);
  let errors = count(&issues, Severity::Error);

  let result = ProfileValidation {
    profile: profile.name.clone(),
    file: file.display().to_string(),
    valid: errors == 0,
    errors,
    warnings: count(&issues, Severity::Warning),
    issues,
  };

  let exit_code = if result.valid {
    ExitCode::Success
  } else {
    ExitCode::Invalid
  };

  Ok(Report::new(&result).with_exit_code(exit_code))
}

impl Render for ProfileValidation {
  fn render_text(&self) -> String {
    let mut text = format!(
      "Profile '{}' ({}): {} error(s), {} warning(s)\n",
      self.profile, self.file, self.errors, self.warnings
    );

    text.push_str(&render_issues(&self.issues));

    if self.valid {
      text.push_str("The profile is valid.\n");
    }

    text
  }
}
