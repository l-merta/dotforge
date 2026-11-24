//! `dotforge system ...` commands: what DotForge sees on this machine.

use clap::Subcommand;

use crate::error::AppError;
use crate::output::{Render, Report};
use crate::system::host::{Host, HostSummary};

#[derive(Subcommand, Debug)]
pub enum SystemCommands {
  /// Show what DotForge detects about this machine
  Show,
}

pub fn handle(command: SystemCommands) -> Result<Report, AppError> {
  match command {
    SystemCommands::Show => Ok(Report::new(&Host::detect().summary())),
  }
}

fn yes_no(value: Option<bool>) -> &'static str {
  match value {
    Some(true) => "yes",
    Some(false) => "no",
    None => "-",
  }
}

impl Render for HostSummary {
  fn render_text(&self) -> String {
    let mut text = String::from("This machine\n────────────────────────────────\n");
    let row = |text: &mut String, label: &str, value: String| {
      text.push_str(&format!("  {:20} {value}\n", format!("{label}:")));
    };

    let name = match (&self.name, &self.version) {
      (Some(name), Some(version)) => format!("{name} {version}"),
      (Some(name), None) => name.clone(),
      _ => "unknown (no /etc/os-release)".into(),
    };
    row(&mut text, "system", name);
    row(
      &mut text,
      "distribution ID",
      self.distribution.clone().unwrap_or("-".into()),
    );
    if !self.based_on.is_empty() {
      row(&mut text, "based on", self.based_on.join(", "));
    }
    row(
      &mut text,
      "DotForge family",
      self
        .family
        .map(|family| family.to_string())
        .unwrap_or_else(|| "not supported yet".into()),
    );
    row(
      &mut text,
      "package manager",
      self.package_manager.clone().unwrap_or("-".into()),
    );
    row(
      &mut text,
      "installed packages",
      self
        .installed_packages
        .map(|n| n.to_string())
        .unwrap_or("-".into()),
    );
    if self.family.is_some() {
      row(
        &mut text,
        "AUR helper",
        self
          .aur_helper
          .clone()
          .unwrap_or_else(|| "none (install paru or yay for AUR packages)".into()),
      );
      row(
        &mut text,
        "repositories",
        self
          .repositories
          .as_ref()
          .map(|repos| repos.join(", "))
          .unwrap_or_else(|| "- (no /etc/pacman.conf)".into()),
      );
    }
    row(
      &mut text,
      "session",
      self.session_type.clone().unwrap_or("-".into()),
    );
    row(&mut text, "systemd", yes_no(Some(self.systemd)).into());

    if let Some(problem) = &self.problem {
      text.push_str(&format!("\nnote: {problem}\n"));
    }

    text
  }
}
