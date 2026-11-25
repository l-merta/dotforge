//! Running a plan, step by step, and reporting what really happened.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::checks::{self, ActionState, Context};
use super::links;
use super::log::Runner;
use super::plan::Plan;
use super::prompt::DroppedChoice;
use super::state::{ManagedEntry, ProfileState};
use crate::system::compare::Comparison;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
  Ok,
  Failed,
  Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
  /// `install`, `aur`, `setup`, `link`, `unlink`, `remove`, `ignore`,
  /// `keep` or `script`.
  pub kind: &'static str,
  pub description: String,
  pub command: Option<String>,
  pub status: StepStatus,
  pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
  pub ok: usize,
  pub failed: usize,
  pub skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
  pub profile: String,
  pub file: String,
  /// `true` only when no step failed.
  pub success: bool,
  pub counts: Counts,
  pub steps: Vec<StepResult>,
  /// Things to do afterwards, e.g. log out and back in.
  pub notes: Vec<String>,
  pub log: Option<String>,
}

pub fn execute(
  plan: &Plan,
  comparison: &Comparison,
  choices: &BTreeMap<String, DroppedChoice>,
  approved_scripts: &BTreeSet<String>,
  runner: &mut dyn Runner,
  ctx: &Context,
  state: &mut ProfileState,
) -> ApplyReport {
  let mut steps = Vec::new();
  let mut failed_entries: BTreeSet<String> = BTreeSet::new();
  let mut notes: BTreeSet<String> = BTreeSet::new();

  runner.note(&format!("profile: {} ({})", plan.profile, plan.file));

  // 1 + 2. Packages: official first, then the AUR.
  for (kind, step) in [("install", &plan.install), ("aur", &plan.aur)] {
    let Some(step) = step else { continue };
    let description = format!(
      "install {} package(s){}: {}",
      step.packages.len(),
      if kind == "aur" { " from the AUR" } else { "" },
      step.packages.join(", ")
    );

    let result = match &step.command {
      None => StepResult {
        kind,
        description,
        command: None,
        status: StepStatus::Failed,
        detail: step.problem.clone(),
      },
      Some(command) => {
        let outcome = runner.run(command);
        StepResult {
          kind,
          description,
          command: Some(command.to_string()),
          status: if outcome.success {
            StepStatus::Ok
          } else {
            StepStatus::Failed
          },
          detail: (!outcome.success).then(|| failure_detail(&outcome)),
        }
      }
    };

    if result.status == StepStatus::Failed {
      failed_entries.extend(step.entries.iter().cloned());
    }
    steps.push(result);
  }

  // 3. Setup actions, in install order.
  for setup in &plan.setup {
    if setup.check == ActionState::Done {
      continue;
    }

    let mut result = StepResult {
      kind: "setup",
      description: format!("{} ({})", setup.description, setup.entry),
      command: setup.command.as_ref().map(ToString::to_string),
      status: StepStatus::Skipped,
      detail: None,
    };

    if failed_entries.contains(&setup.entry) {
      result.detail = Some(format!("installing {} failed", setup.entry));
    } else if let ActionState::Unknown(reason) = &setup.check {
      result.detail = Some(format!("can't be checked here: {reason}"));
    } else if let Some(command) = &setup.command {
      match checks::precheck(&setup.action, ctx) {
        Err(problem) => {
          result.status = StepStatus::Failed;
          result.detail = Some(problem);
        }
        Ok(()) => {
          let outcome = runner.run(command);
          if outcome.success {
            result.status = StepStatus::Ok;
            if let Some(note) = checks::after_note(&setup.action) {
              notes.insert(format!("{}: {note}", setup.description));
            }
          } else {
            result.status = StepStatus::Failed;
            result.detail = Some(failure_detail(&outcome));
          }
        }
      }
    }

    steps.push(result);
  }

  // 3b. Config files and assets: links into your home folder.
  let mut made = BTreeMap::new();
  for step in &plan.links {
    if !step.needs_change() {
      made.insert(step.link.target.clone(), step.link.source.clone());
      continue;
    }
    let description = format!("link {}", step.link.label);
    let detail_ok = match &step.state {
      links::TargetState::Replace { found, backup } => {
        Some(format!("{found} was there; moved it to {backup}"))
      }
      _ => None,
    };
    runner.note(&format!(
      "link {} -> {}",
      step.link.target.display(),
      step.link.source.display()
    ));
    let result = links::make(step);
    if let Err(error) = &result {
      runner.note(&format!("! {error}"));
    } else {
      made.insert(step.link.target.clone(), step.link.source.clone());
      if step.link.label == "fonts" {
        notes.insert("new fonts: run 'fc-cache -f' if an app doesn't see them yet".into());
      }
    }
    steps.push(StepResult {
      kind: "link",
      description,
      command: None,
      status: if result.is_ok() {
        StepStatus::Ok
      } else {
        StepStatus::Failed
      },
      detail: result.err().or(detail_ok),
    });
  }
  let mut unlinked = Vec::new();
  for step in &plan.unlinks {
    let result = links::unmake(step);
    if result.is_ok() {
      unlinked.push(step.target.clone());
    }
    steps.push(StepResult {
      kind: "unlink",
      description: format!(
        "remove the link {} (no longer in the profile)",
        step.target.display()
      ),
      command: None,
      status: if result.is_ok() {
        StepStatus::Ok
      } else {
        StepStatus::Failed
      },
      detail: result.err(),
    });
  }
  links::remember(state, &made, &unlinked);
  // Links DotForge no longer sees (removed from the profile and already
  // gone, or replaced by you) are forgotten.
  state.links.retain(|target, _| {
    made.contains_key(std::path::Path::new(target))
      || plan
        .unlinks
        .iter()
        .any(|u| u.target == std::path::Path::new(target) && !unlinked.contains(&u.target))
  });

  // 4. Entries dropped from the profile: only what you chose.
  let mut forget: Vec<String> = Vec::new();
  for dropped in &plan.dropped {
    if dropped.still_installed.is_empty() {
      forget.push(dropped.id.clone());
      continue; // nothing left on the machine, nothing to ask
    }

    let choice = choices
      .get(&dropped.id)
      .copied()
      .unwrap_or(DroppedChoice::Keep);
    let result = match choice {
      DroppedChoice::Keep => StepResult {
        kind: "keep",
        description: format!(
          "keep {} for now (you'll be asked again next time)",
          dropped.id
        ),
        command: None,
        status: StepStatus::Skipped,
        detail: None,
      },
      DroppedChoice::Ignore => {
        forget.push(dropped.id.clone());
        StepResult {
          kind: "ignore",
          description: format!(
            "leave {} installed, no longer managed by DotForge",
            dropped.id
          ),
          command: None,
          status: StepStatus::Ok,
          detail: None,
        }
      }
      DroppedChoice::Remove => match &dropped.remove_command {
        None => {
          forget.push(dropped.id.clone());
          StepResult {
            kind: "remove",
            description: format!("remove {}", dropped.id),
            command: None,
            status: StepStatus::Ok,
            detail: Some("its packages are still needed by the profile, so they stay".into()),
          }
        }
        Some(command) => {
          let outcome = runner.run(command);
          if outcome.success {
            forget.push(dropped.id.clone());
          }
          StepResult {
            kind: "remove",
            description: format!("remove {}: {}", dropped.id, dropped.removable.join(", ")),
            command: Some(command.to_string()),
            status: if outcome.success {
              StepStatus::Ok
            } else {
              StepStatus::Failed
            },
            detail: (!outcome.success).then(|| failure_detail(&outcome)),
          }
        }
      },
    };
    steps.push(result);
  }

  // 5. Your own scripts, last, and only the ones you said yes to.
  let earlier_failure = steps.iter().any(|s| s.status == StepStatus::Failed);
  for script in plan.scripts.iter().filter(|s| s.wants_to_run()) {
    let Some(command) = &script.command else {
      continue;
    };
    let description = format!("run your script {} ({})", script.path, script.status);

    let skip = if earlier_failure {
      Some("an earlier step failed, so your scripts were not run")
    } else if !approved_scripts.contains(&script.path) {
      Some("you didn't confirm it; it will be asked again next time")
    } else {
      None
    };
    if let Some(reason) = skip {
      steps.push(StepResult {
        kind: "script",
        description,
        command: Some(command.to_string()),
        status: StepStatus::Skipped,
        detail: Some(reason.into()),
      });
      continue;
    }

    let outcome = runner.run(command);
    if outcome.success {
      state
        .scripts
        .insert(script.path.clone(), script.sha256.clone());
    }
    steps.push(StepResult {
      kind: "script",
      description,
      command: Some(command.to_string()),
      status: if outcome.success {
        StepStatus::Ok
      } else {
        StepStatus::Failed
      },
      detail: (!outcome.success).then(|| failure_detail(&outcome)),
    });
  }
  // Scripts taken out of the profile are forgotten.
  state
    .scripts
    .retain(|path, _| plan.scripts.iter().any(|s| &s.path == path));

  // 6. Remember what DotForge now manages for this profile.
  for entry in &comparison.entries {
    if failed_entries.contains(&entry.id) || entry.reason.is_some() {
      continue;
    }
    let had_missing = !entry.missing_packages.is_empty() || !entry.missing_aur.is_empty();
    let previous = state
      .entries
      .get(&entry.id)
      .is_some_and(|e| e.installed_by_dotforge);
    state.entries.insert(
      entry.id.clone(),
      ManagedEntry {
        packages: entry.packages.clone(),
        aur: entry.aur.clone(),
        installed_by_dotforge: previous || had_missing,
      },
    );
  }
  for id in forget {
    state.entries.remove(&id);
  }

  let mut counts = Counts::default();
  for step in &steps {
    match step.status {
      StepStatus::Ok => counts.ok += 1,
      StepStatus::Failed => counts.failed += 1,
      StepStatus::Skipped => counts.skipped += 1,
    }
  }
  runner.note(&format!(
    "finished: {} ok, {} failed, {} skipped",
    counts.ok, counts.failed, counts.skipped
  ));

  ApplyReport {
    profile: plan.profile.clone(),
    file: plan.file.clone(),
    success: counts.failed == 0,
    counts,
    steps,
    notes: notes.into_iter().collect(),
    log: runner.log_path().map(|p| p.display().to_string()),
  }
}

fn failure_detail(outcome: &super::log::RunOutcome) -> String {
  if let Some(error) = &outcome.error {
    return error.clone();
  }
  let code = outcome
    .exit_code
    .map_or("it was stopped".to_string(), |c| format!("exit code {c}"));
  match outcome.tail.last() {
    Some(last) => format!("{code}: {last}"),
    None => code,
  }
}
