//! Building the plan: everything `apply` would do, worked out before
//! anything is changed. `--dry-run` shows exactly this.

use std::collections::BTreeSet;

use serde::Serialize;

use super::checks::{self, ActionState, Context};
use super::command::CommandSpec;
use super::links::{LinkStep, UnlinkStep};
use super::state::ProfileState;
use crate::catalog::model::SetupAction;
use crate::system::compare::{Comparison, Status};
use crate::system::host::Host;
use crate::validation::Issue;

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
  pub profile: String,
  pub file: String,
  /// Your home folder, for showing paths as `~/...`.
  #[serde(skip)]
  pub home: String,
  pub host: String,
  /// Entries that are already fully installed.
  pub already_installed: Vec<String>,
  pub install: Option<PackageStep>,
  pub aur: Option<PackageStep>,
  pub setup: Vec<SetupStep>,
  /// Entries DotForge managed for this profile that the profile no longer
  /// has.
  pub dropped: Vec<DroppedEntry>,
  /// The profile's own scripts, in profile order.
  pub scripts: Vec<ScriptStep>,
  /// Config files and assets linked from the profile folder.
  pub links: Vec<LinkStep>,
  /// Links DotForge made earlier that the profile no longer has.
  pub unlinks: Vec<UnlinkStep>,
  pub notes: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptStep {
  /// As written in the profile, e.g. `scripts/tweaks.sh`.
  pub path: String,
  /// `new`, `changed` or `unchanged` (already ran with these contents).
  pub status: &'static str,
  pub sha256: String,
  pub lines: usize,
  /// The command, if the script would run.
  pub command: Option<CommandSpec>,
}

impl ScriptStep {
  pub fn wants_to_run(&self) -> bool {
    self.command.is_some()
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageStep {
  pub packages: Vec<String>,
  /// The entries these packages belong to.
  pub entries: Vec<String>,
  /// `None` when it can't run, e.g. AUR packages without an AUR helper.
  pub command: Option<CommandSpec>,
  pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupStep {
  pub entry: String,
  pub action: SetupAction,
  pub description: String,
  #[serde(flatten)]
  pub check: ActionState,
  /// The command, if the action still needs doing.
  pub command: Option<CommandSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DroppedEntry {
  pub id: String,
  /// The packages DotForge remembers for it that are still installed.
  pub still_installed: Vec<String>,
  /// The packages that would be uninstalled if you choose "remove": those
  /// still installed and not needed by anything else in the profile.
  pub removable: Vec<String>,
  pub installed_by_dotforge: bool,
  pub remove_command: Option<CommandSpec>,
}

impl Plan {
  /// Steps that change the system (not counting dropped entries, which are
  /// only changed if you say so).
  pub fn has_changes(&self) -> bool {
    self.install.is_some()
      || self.aur.is_some()
      || self.setup.iter().any(|s| s.command.is_some())
      || self.links.iter().any(LinkStep::needs_change)
      || !self.unlinks.is_empty()
  }

  pub fn is_empty(&self) -> bool {
    !self.has_changes()
      && self.dropped.is_empty()
      && !self.scripts.iter().any(ScriptStep::wants_to_run)
  }
}

/// Adds the profile's custom scripts to the plan. Scripts were already
/// checked by validation (inside the profile folder, readable).
pub fn add_scripts(
  plan: &mut Plan,
  profile_dir: &std::path::Path,
  scripts: &[String],
  previous: Option<&ProfileState>,
) {
  use sha2::{Digest, Sha256};

  for path in scripts {
    let full = profile_dir.join(path);
    let contents = std::fs::read(&full).unwrap_or_default();
    let sha256: String = Sha256::digest(&contents)
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect();
    let status = match previous.and_then(|p| p.scripts.get(path)) {
      None => "new",
      Some(old) if *old != sha256 => "changed",
      Some(_) => "unchanged",
    };
    let command = (status != "unchanged").then(|| {
      CommandSpec::user("bash", [], &[full.display().to_string()])
        .in_dir(&profile_dir.display().to_string())
    });

    plan.scripts.push(ScriptStep {
      path: path.clone(),
      status,
      sha256,
      lines: String::from_utf8_lossy(&contents).lines().count(),
      command,
    });
  }
}

pub fn build(
  comparison: &Comparison,
  host: &Host,
  ctx: &Context,
  previous: Option<&ProfileState>,
) -> Plan {
  let backend = host
    .backend
    .as_ref()
    .expect("apply only runs on supported systems");
  let mut notes = comparison.notes.clone();

  let mut repo_entries = Vec::new();
  let mut aur_entries = Vec::new();
  let mut already_installed = Vec::new();
  for entry in &comparison.entries {
    if entry.status == Status::Installed {
      already_installed.push(entry.id.clone());
    }
    if !entry.missing_packages.is_empty() {
      repo_entries.push(entry.id.clone());
    }
    if !entry.missing_aur.is_empty() {
      aur_entries.push(entry.id.clone());
    }
  }

  let packages = &comparison.to_install.packages;
  let install = (!packages.is_empty()).then(|| PackageStep {
    packages: packages.clone(),
    entries: repo_entries,
    command: Some(backend.install_command(packages)),
    problem: None,
  });

  let aur_packages = &comparison.to_install.aur;
  let aur = (!aur_packages.is_empty()).then(|| {
    let command = backend.aur_install_command(aur_packages);
    PackageStep {
      packages: aur_packages.clone(),
      entries: aur_entries,
      problem: command
        .is_none()
        .then(|| "no AUR helper found: install paru or yay first".to_string()),
      command,
    }
  });

  let setup = comparison
    .entries
    .iter()
    .flat_map(|entry| entry.setup.iter().map(move |action| (entry, action)))
    .map(|(entry, action)| {
      let check = checks::check(action, ctx);
      let command = match &check {
        ActionState::Done => None,
        ActionState::Needed => Some(checks::command_for(action, ctx)),
        ActionState::Unknown(_) => None,
      };
      SetupStep {
        entry: entry.id.clone(),
        action: action.clone(),
        description: action.to_string(),
        check,
        command,
      }
    })
    .collect();

  // Packages the profile still needs must never be removed.
  let wanted: BTreeSet<&str> = comparison.entries.iter().map(|e| e.id.as_str()).collect();
  let mut needed_packages: BTreeSet<String> = BTreeSet::new();
  if let Some(previous) = previous {
    for (id, managed) in &previous.entries {
      if wanted.contains(id.as_str()) {
        needed_packages.extend(managed.packages.iter().chain(&managed.aur).cloned());
      }
    }
  }
  needed_packages.extend(packages.iter().cloned());
  needed_packages.extend(aur_packages.iter().cloned());

  let dropped = previous
    .map(|previous| {
      previous
        .entries
        .iter()
        .filter(|(id, _)| !wanted.contains(id.as_str()))
        .map(|(id, managed)| {
          let still_installed: Vec<String> = managed
            .packages
            .iter()
            .chain(&managed.aur)
            .filter(|package| backend.is_installed(package))
            .cloned()
            .collect();
          let removable: Vec<String> = still_installed
            .iter()
            .filter(|package| !needed_packages.contains(*package))
            .cloned()
            .collect();
          DroppedEntry {
            id: id.clone(),
            remove_command: (!removable.is_empty()).then(|| backend.remove_command(&removable)),
            still_installed,
            removable,
            installed_by_dotforge: managed.installed_by_dotforge,
          }
        })
        .collect()
    })
    .unwrap_or_default();

  if host.root != std::path::Path::new("/") {
    notes.push(Issue::warning(
      "DOTFORGE_SYSTEM_ROOT",
      format!(
        "checks read {} instead of /, but commands still change the real system",
        host.root.display()
      ),
    ));
  }

  Plan {
    profile: comparison.profile.clone(),
    file: comparison.file.clone(),
    home: String::new(),
    host: comparison.host.clone(),
    already_installed,
    install,
    aur,
    setup,
    dropped,
    scripts: Vec::new(),
    links: Vec::new(),
    unlinks: Vec::new(),
    notes,
  }
}
