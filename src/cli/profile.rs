//! `dotforge profile ...` commands.

use clap::Subcommand;
use serde::Serialize;

use crate::catalog::catalog::Catalog;
use crate::editor;
use crate::error::AppError;
use crate::exit_code::ExitCode;
use crate::output::{Render, Report};
use crate::paths;
use crate::profile::discovery::{self, Discovery, Status};
use crate::profile::model::{DeviceType, Profile};
use crate::profile::template::{self, NewProfile};
use crate::profile::{loader, validate};
use crate::settings;
use crate::system::compare::{self, Comparison, Status as EntryStatus};
use crate::system::host::Host;
use crate::validation::{Issue, Severity, count, render_issues};

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
  /// List your profiles (in ~/.config/dotforge/profiles/)
  List,

  /// Show what a profile contains
  Show {
    /// A profile name, or a path to a profile folder or profile.yml
    profile: String,
  },

  /// Create a new profile from a starter template
  Create {
    /// Name of the new profile folder, e.g. desktop or work-laptop
    name: String,

    /// Display name written into profile.yml (default: the folder name)
    #[arg(long)]
    display_name: Option<String>,

    /// A short description
    #[arg(long)]
    description: Option<String>,

    /// What kind of machine this profile is for
    #[arg(long, value_enum, default_value = "desktop")]
    device: DeviceType,
  },

  /// Open a profile in your editor (set it with 'dotforge config edit')
  Edit {
    /// A profile name, or a path to a profile folder or profile.yml
    profile: String,
  },

  /// Check a profile for mistakes
  Validate {
    /// A profile name, or a path to a profile folder or profile.yml
    profile: String,
  },

  /// Compare a profile with this machine: what is installed, what is missing
  Status {
    /// A profile name, or a path to a profile folder or profile.yml
    profile: String,
  },

  /// Make this machine match a profile: shows a plan, asks, then installs
  Apply {
    /// A profile name, or a path to a profile folder or profile.yml
    profile: String,

    /// Only show what would be done; change nothing
    #[arg(long)]
    dry_run: bool,

    /// Don't ask before applying (dropped items are still never removed)
    #[arg(long, short = 'y')]
    yes: bool,
  },
}

pub fn handle(command: ProfileCommands) -> Result<Report, AppError> {
  match command {
    ProfileCommands::List => {
      let found = discovery::discover(&paths::profiles_dir()?)?;
      Ok(Report::new(&found))
    }

    ProfileCommands::Show { profile } => show(&profile),

    ProfileCommands::Create {
      name,
      display_name,
      description,
      device,
    } => create(NewProfile {
      name: &name,
      display_name: display_name.as_deref(),
      description: description.as_deref(),
      device_type: device,
    }),

    ProfileCommands::Edit { profile } => edit(&profile),

    ProfileCommands::Validate { profile } => validate_profile(&profile),

    ProfileCommands::Status { profile } => status(&profile),

    ProfileCommands::Apply {
      profile,
      dry_run,
      yes,
    } => crate::cli::apply::apply(&profile, crate::cli::apply::Options { dry_run, yes }),
  }
}

// ---------- list ----------

impl Render for Discovery {
  fn render_text(&self) -> String {
    if self.profiles.is_empty() {
      return format!(
        "No profiles yet in {}.\nCreate one with: dotforge profile create <name>\n",
        self.profiles_dir
      );
    }

    let mut text = format!("Profiles in {}:\n\n", self.profiles_dir);
    let width = self
      .profiles
      .iter()
      .map(|p| p.name.len())
      .max()
      .unwrap_or(0);

    for profile in &self.profiles {
      let detail = match profile.status {
        Status::Ok => {
          let mut detail = profile.display_name.clone().unwrap_or_default();
          if let Some(description) = profile.description.as_deref().filter(|d| !d.is_empty()) {
            detail.push_str(&format!(" - {description}"));
          }
          detail
        }
        _ => format!(
          "[problem] {}",
          profile.problem.as_deref().unwrap_or("unknown problem")
        ),
      };
      text.push_str(&format!("  {:width$}  {detail}\n", profile.name));
    }

    if !self.issues.is_empty() {
      text.push('\n');
      text.push_str(&render_issues(&self.issues));
    }

    text
  }
}

// ---------- show ----------

#[derive(Serialize)]
struct ShownProfile {
  file: String,
  profile: Profile,
}

fn show(name_or_path: &str) -> Result<Report, AppError> {
  let file = loader::resolve(name_or_path, &paths::profiles_dir()?)?;
  let profile = loader::load(&file)?;

  Ok(Report::new(&ShownProfile {
    file: file.display().to_string(),
    profile,
  }))
}

impl Render for ShownProfile {
  fn render_text(&self) -> String {
    let p = &self.profile;
    let mut text = String::new();

    text.push_str(&format!("{}\n", p.name));
    text.push_str("────────────────────────────────\n");
    if !p.description.is_empty() {
      text.push_str(&format!("{}\n", p.description));
    }
    text.push_str(&format!("File: {}\n\n", self.file));

    text.push_str("Device:\n");
    line(&mut text, "type", Some(p.device.kind.to_string()));
    line(&mut text, "os family", Some(p.device.os.family.to_string()));
    line(
      &mut text,
      "distribution",
      Some(p.device.os.distribution.clone()),
    );

    text.push_str("\nSystem:\n");
    line(
      &mut text,
      "audio",
      p.system.audio.as_ref().map(|a| a.backend.clone()),
    );
    line(
      &mut text,
      "graphics drivers",
      list(&p.system.graphics.drivers),
    );
    line(&mut text, "graphics api", list(&p.system.graphics.api));
    line(
      &mut text,
      "hardware drivers",
      list(&p.system.hardware.drivers),
    );

    text.push_str("\nApplications:\n");
    line(&mut text, "terminal", p.applications.terminal.clone());
    for (group, ids) in p.applications.groups() {
      line(&mut text, group, list(ids));
    }

    text.push_str("\nRice:\n");
    line(&mut text, "desktop", p.rice.desktop.clone());
    line(&mut text, "compositor", p.rice.compositor.clone());
    line(&mut text, "shell", p.rice.shell.clone());
    for (slot, id) in p.rice.components.all() {
      line(&mut text, slot, id.clone());
    }

    text
  }
}

fn line(text: &mut String, label: &str, value: Option<String>) {
  let value = value.unwrap_or_else(|| "-".into());
  text.push_str(&format!("  {:18} {value}\n", format!("{label}:")));
}

fn list(items: &[String]) -> Option<String> {
  if items.is_empty() {
    None
  } else {
    Some(items.join(", "))
  }
}

// ---------- create ----------

#[derive(Serialize)]
struct CreatedProfile {
  name: String,
  path: String,
  file: String,
}

fn create(new: NewProfile) -> Result<Report, AppError> {
  let folder = template::create(&paths::profiles_dir()?, &new)?;

  Ok(Report::new(&CreatedProfile {
    name: new.name.to_string(),
    path: folder.display().to_string(),
    file: folder.join(loader::PROFILE_FILE).display().to_string(),
  }))
}

impl Render for CreatedProfile {
  fn render_text(&self) -> String {
    format!(
      "Created profile '{name}' in {path}\n\nNext steps:\n  1. Edit {file}\n  2. Check it:  dotforge profile validate {name}\n  3. See it:    dotforge profile show {name}\n",
      name = self.name,
      path = self.path,
      file = self.file,
    )
  }
}

// ---------- edit ----------

#[derive(Serialize)]
struct EditedProfile {
  file: String,
  editor: String,
  /// The check that runs right after the editor closes.
  validation: ProfileValidation,
}

fn edit(name_or_path: &str) -> Result<Report, AppError> {
  let file = loader::resolve(name_or_path, &paths::profiles_dir()?)?;
  let editor_command = settings::load()?.editor().to_string();

  editor::open(&editor_command, &file)?;

  // Check the profile straight away, so mistakes show up now.
  let validation = check(&file)?;

  Ok(Report::new(&EditedProfile {
    file: file.display().to_string(),
    editor: editor_command,
    validation,
  }))
}

impl Render for EditedProfile {
  fn render_text(&self) -> String {
    format!(
      "Saved {}.
{}",
      self.file,
      self.validation.render_text()
    )
  }
}

// ---------- status ----------

fn status(name_or_path: &str) -> Result<Report, AppError> {
  let file = loader::resolve(name_or_path, &paths::profiles_dir()?)?;
  let validation = check(&file)?;

  if !validation.valid {
    return Err(AppError::Invalid {
      message: format!(
        "the profile '{}' has {} error(s), so it can't be compared yet",
        validation.profile, validation.errors
      ),
      hint: Some(format!(
        "run 'dotforge profile validate {name_or_path}' to see them"
      )),
    });
  }

  let profile = loader::load(&file)?;
  let catalog = Catalog::load()?;
  let host = Host::detect();
  let comparison = compare::compare(&profile, &file.display().to_string(), &catalog, &host)?;

  Ok(Report::new(&comparison))
}

impl Render for Comparison {
  fn render_text(&self) -> String {
    let mut text = format!(
      "Profile '{}' compared with this machine ({}):\n\n",
      self.profile, self.host
    );

    for entry in &self.entries {
      let (mark, detail) = match entry.status {
        EntryStatus::Installed => ("ok  ", "installed".to_string()),
        EntryStatus::Missing => {
          let mut packages = entry.missing_packages.clone();
          packages.extend(entry.missing_aur.iter().map(|p| format!("{p} (AUR)")));
          ("need", format!("missing: {}", packages.join(", ")))
        }
        EntryStatus::Unsupported => ("??  ", "can't check here".to_string()),
      };

      let why = if entry
        .wanted_by
        .iter()
        .all(|w| w.starts_with("dependency of"))
      {
        format!("  ({})", entry.wanted_by.join(", "))
      } else {
        String::new()
      };

      text.push_str(&format!("  [{mark}] {:28} {detail}{why}\n", entry.id));
    }

    text.push_str(&format!(
      "\nSummary: {} installed, {} missing, {} can't be checked here.\n",
      self.counts.installed, self.counts.missing, self.counts.unsupported
    ));

    if self.counts.unsupported > 0
      && let Some(reason) = self.entries.iter().find_map(|e| e.reason.as_deref())
    {
      text.push_str(&format!("Why: {reason}\n"));
    }

    let install = &self.to_install;
    if !install.packages.is_empty() || !install.aur.is_empty() {
      text.push_str("\nTo match the profile, DotForge would install:\n");
      if !install.packages.is_empty() {
        let manager = install
          .package_manager
          .as_deref()
          .unwrap_or("the package manager");
        text.push_str(&format!(
          "  with {manager}: {}\n",
          install.packages.join(" ")
        ));
      }
      if !install.aur.is_empty() {
        let helper = install
          .aur_helper
          .as_deref()
          .unwrap_or("an AUR helper (paru or yay)");
        text.push_str(&format!("  with {helper}: {}\n", install.aur.join(" ")));
      }
    }

    let setup: Vec<String> = self
      .entries
      .iter()
      .flat_map(|e| {
        e.setup
          .iter()
          .map(move |action| format!("{} ({})", action, e.id))
      })
      .collect();
    if !setup.is_empty() {
      text.push_str("\nSetup steps (checked when applying, from Milestone 5):\n");
      for step in setup {
        text.push_str(&format!("  - {step}\n"));
      }
    }

    if !self.notes.is_empty() {
      text.push('\n');
      text.push_str(&render_issues(&self.notes));
    }

    if self.satisfied {
      text.push_str("\nThis machine already matches the profile.\n");
    } else {
      text.push_str("\nNothing was changed. This only shows what is different.\n");
    }

    text
  }
}

// ---------- validate ----------

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
  let result = check(&file)?;

  let exit_code = if result.valid {
    ExitCode::Success
  } else {
    ExitCode::Invalid
  };

  Ok(Report::new(&result).with_exit_code(exit_code))
}

fn check(file: &std::path::Path) -> Result<ProfileValidation, AppError> {
  let profile = loader::load(file)?;
  let catalog = Catalog::load()?;

  let mut issues = validate::validate(&profile, &catalog);
  if let Some(dir) = file.parent() {
    issues.extend(validate::validate_scripts(&profile, dir));
    issues.extend(crate::apply::links::check(&profile, dir, &catalog));
  }
  let errors = count(&issues, Severity::Error);

  let result = ProfileValidation {
    profile: profile.name.clone(),
    file: file.display().to_string(),
    valid: errors == 0,
    errors,
    warnings: count(&issues, Severity::Warning),
    issues,
  };

  Ok(result)
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
