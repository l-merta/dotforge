//! `dotforge profile apply`: make this machine match a profile.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::apply::checks::{self, ActionState, Context};
use crate::apply::engine::{self, ApplyReport, StepStatus};
use crate::apply::log::RealRunner;
use crate::apply::plan::{self, Plan};
use crate::apply::prompt::{Prompter, StdinPrompter, YesPrompter};
use crate::apply::state;
use crate::catalog::catalog::Catalog;
use crate::error::AppError;
use crate::exit_code::ExitCode;
use crate::output::{Render, Report};
use crate::paths;
use crate::profile::{loader, validate};
use crate::system::compare;
use crate::system::host::Host;
use crate::validation::{Severity, count, render_issues};

pub struct Options {
  pub dry_run: bool,
  pub yes: bool,
}

#[derive(Serialize)]
struct Outcome {
  /// `dry_run`, `nothing_to_do`, `cancelled` or `applied`.
  result: &'static str,
  plan: Plan,
  report: Option<ApplyReport>,
}

pub fn apply(name_or_path: &str, options: Options) -> Result<Report, AppError> {
  refuse_root()?;

  let file = loader::resolve(name_or_path, &paths::profiles_dir()?)?;
  let profile = loader::load(&file)?;
  let catalog = Catalog::load()?;

  let issues = validate::validate(&profile, &catalog);
  let errors = count(&issues, Severity::Error);
  if errors > 0 {
    return Err(AppError::Invalid {
      message: format!(
        "the profile '{}' has {errors} error(s), so it can't be applied",
        profile.name
      ),
      hint: Some(format!(
        "run 'dotforge profile validate {name_or_path}' to see them"
      )),
    });
  }

  let host = Host::detect();
  if let Some(reason) = host.unsupported_reason() {
    return Err(AppError::Invalid {
      message: format!("can't apply here: {reason}"),
      hint: None,
    });
  }

  let file_key = std::fs::canonicalize(&file)
    .unwrap_or(file.clone())
    .display()
    .to_string();
  let comparison = compare::compare(&profile, &file_key, &catalog, &host)?;

  let ctx = Context {
    root: host.root.clone(),
    user: checks::current_user().ok_or_else(|| AppError::Invalid {
      message: "could not tell which user you are ($USER is not set)".into(),
      hint: None,
    })?,
    systemd: host.systemd,
  };

  let state_dir = paths::state_dir()?;
  let mut saved = state::load(&state_dir)?;
  let previous = saved.profiles.get(&file_key);
  let plan = plan::build(&comparison, &host, &ctx, previous);

  if options.dry_run {
    return Ok(Report::new(&Outcome {
      result: "dry_run",
      plan,
      report: None,
    }));
  }

  // Show the plan, then ask.
  let mut prompter: Box<dyn Prompter> = if options.yes {
    Box::new(YesPrompter)
  } else {
    Box::new(StdinPrompter)
  };

  let mut choices = BTreeMap::new();
  if plan.has_changes() || plan.dropped.iter().any(|d| !d.still_installed.is_empty()) {
    eprint!("{}", plan.render_text());

    if plan.has_changes() && !prompter.confirm("\nApply these changes?") {
      return Ok(
        Report::new(&Outcome {
          result: "cancelled",
          plan,
          report: None,
        })
        .with_exit_code(ExitCode::Cancelled),
      );
    }

    for dropped in plan
      .dropped
      .iter()
      .filter(|d| !d.still_installed.is_empty())
    {
      let choice = prompter.dropped(&format!(
        "\n'{}' is no longer in the profile but still installed ({}). What should DotForge do?",
        dropped.id,
        dropped.still_installed.join(", ")
      ));
      choices.insert(dropped.id.clone(), choice);
    }
  }

  let mut runner = RealRunner::new(&state_dir, true);
  let profile_state = saved.profiles.entry(file_key).or_default();
  let report = engine::execute(
    &plan,
    &comparison,
    &choices,
    &mut runner,
    &ctx,
    profile_state,
  );
  state::save(&state_dir, &saved)?;

  let nothing = plan.is_empty();
  let exit_code = if report.success {
    ExitCode::Success
  } else {
    ExitCode::Failure
  };

  Ok(
    Report::new(&Outcome {
      result: if nothing { "nothing_to_do" } else { "applied" },
      plan,
      report: Some(report),
    })
    .with_exit_code(exit_code),
  )
}

/// DotForge asks for administrator rights (sudo) only for the steps that
/// need it. Running all of it as root is refused: AUR helpers don't work
/// as root, and files would end up owned by root.
fn refuse_root() -> Result<(), AppError> {
  let is_root = unsafe { libc::geteuid() } == 0;
  let allowed = std::env::var("DOTFORGE_ALLOW_ROOT").is_ok_and(|v| v == "1");

  if is_root && !allowed {
    return Err(AppError::Invalid {
      message: "don't run 'dotforge profile apply' as root or with sudo".into(),
      hint: Some(
        "run it as your normal user; DotForge uses sudo only for the steps that need it".into(),
      ),
    });
  }
  Ok(())
}

impl Render for Plan {
  fn render_text(&self) -> String {
    let mut text = format!("Plan for '{}' on {}:\n", self.profile, self.host);

    if !self.already_installed.is_empty() {
      text.push_str(&format!(
        "\n  Already installed: {}\n",
        self.already_installed.join(", ")
      ));
    }

    for (title, step) in [
      ("Install", &self.install),
      ("Install from the AUR", &self.aur),
    ] {
      if let Some(step) = step {
        text.push_str(&format!(
          "\n  {title} ({}):\n    {}\n",
          step.packages.len(),
          step.packages.join(" ")
        ));
        match (&step.command, &step.problem) {
          (Some(command), _) => text.push_str(&format!("    runs: {command}\n")),
          (None, Some(problem)) => text.push_str(&format!("    can't run: {problem}\n")),
          _ => {}
        }
      }
    }

    if !self.setup.is_empty() {
      text.push_str("\n  Setup steps:\n");
      for step in &self.setup {
        let status = match &step.check {
          ActionState::Done => "already done".to_string(),
          ActionState::Needed => match &step.command {
            Some(command) => format!("runs: {command}"),
            None => "to do".into(),
          },
          ActionState::Unknown(reason) => format!("skipped, can't check: {reason}"),
        };
        text.push_str(&format!(
          "    - {} ({}): {status}\n",
          step.description, step.entry
        ));
      }
    }

    for dropped in &self.dropped {
      if dropped.still_installed.is_empty() {
        text.push_str(&format!(
          "\n  '{}' was removed from the profile and is no longer installed; DotForge will forget it.\n",
          dropped.id
        ));
      } else {
        text.push_str(&format!(
          "\n  '{}' was removed from the profile but is still installed ({}).\n    You'll be asked: remove, ignore, or keep for now.{}\n",
          dropped.id,
          dropped.still_installed.join(", "),
          match &dropped.remove_command {
            Some(command) => format!("\n    remove runs: {command}"),
            None => "\n    (its packages are still needed by the profile, so nothing would be uninstalled)".into(),
          }
        ));
      }
    }

    if !self.notes.is_empty() {
      text.push('\n');
      text.push_str(&render_issues(&self.notes));
    }

    if self.is_empty() {
      text.push_str("\n  Nothing to do: this machine already matches the profile.\n");
    }

    text
  }
}

impl Render for ApplyReport {
  fn render_text(&self) -> String {
    let mut text = String::from("\nResult:\n");

    for step in &self.steps {
      let mark = match step.status {
        StepStatus::Ok => "ok     ",
        StepStatus::Failed => "FAILED ",
        StepStatus::Skipped => "skipped",
      };
      text.push_str(&format!("  [{mark}] {}\n", step.description));
      if let Some(detail) = &step.detail {
        text.push_str(&format!("            {detail}\n"));
      }
    }

    text.push_str(&format!(
      "\n{} ok, {} failed, {} skipped.\n",
      self.counts.ok, self.counts.failed, self.counts.skipped
    ));

    for note in &self.notes {
      text.push_str(&format!("note: {note}\n"));
    }
    if let Some(log) = &self.log {
      text.push_str(&format!("Full log: {log}\n"));
    }

    if self.success {
      text.push_str("The profile was applied.\n");
    } else {
      text.push_str("Some steps failed, so the profile is NOT fully applied. Fix the problem and run apply again; finished steps are skipped next time.\n");
    }

    text
  }
}

impl Render for Outcome {
  fn render_text(&self) -> String {
    match self.result {
      "dry_run" => format!(
        "{}\nDry run: nothing was changed.\n",
        self.plan.render_text()
      ),
      "cancelled" => "Cancelled. Nothing was changed.\n".to_string(),
      _ => match &self.report {
        Some(report) if self.result == "nothing_to_do" && report.steps.is_empty() => {
          "This machine already matches the profile. Nothing to do.\n".to_string()
        }
        Some(report) => report.render_text(),
        None => String::new(),
      },
    }
  }
}
