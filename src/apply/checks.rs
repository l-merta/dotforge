//! Checking whether a typed setup action is already done, and building the
//! command that does it.
//!
//! Every check only reads: `/etc/group`, `/etc/passwd`, `/etc/shells` (under
//! the system root) or `systemctl is-enabled`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use super::command::CommandSpec;
use crate::catalog::model::SetupAction;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "reason", rename_all = "snake_case")]
pub enum ActionState {
  /// Nothing to do.
  Done,
  /// Needs to be done.
  Needed,
  /// Can't be checked here; the reason says why.
  Unknown(String),
}

/// What the checks need to know about the machine.
pub struct Context {
  pub root: PathBuf,
  pub user: String,
  /// Is systemd running? (Not inside most containers.)
  pub systemd: bool,
}

/// The user DotForge runs as: `$USER`, or the answer of `id -un`.
pub fn current_user() -> Option<String> {
  if let Ok(user) = std::env::var("USER")
    && !user.is_empty()
  {
    return Some(user);
  }

  let output = Command::new("id").arg("-un").output().ok()?;
  let user = String::from_utf8_lossy(&output.stdout).trim().to_string();
  (!user.is_empty()).then_some(user)
}

pub fn check(action: &SetupAction, ctx: &Context) -> ActionState {
  match action {
    SetupAction::EnableService { service, user } => service_enabled(service, *user),
    SetupAction::AddUserToGroup { group } => {
      if group_members(&ctx.root, group).is_some_and(|members| members.contains(&ctx.user)) {
        ActionState::Done
      } else {
        ActionState::Needed
      }
    }
    SetupAction::SetDefaultShell { shell } => {
      if login_shell(&ctx.root, &ctx.user).as_deref() == Some(shell.as_str()) {
        ActionState::Done
      } else {
        ActionState::Needed
      }
    }
  }
}

/// The command that performs `action`.
pub fn command_for(action: &SetupAction, ctx: &Context) -> CommandSpec {
  match action {
    SetupAction::EnableService {
      service,
      user: false,
    } => {
      let mut flags = vec!["enable"];
      // Starting needs a running systemd; enabling works without it.
      if ctx.systemd {
        flags.push("--now");
      }
      CommandSpec::sudo("systemctl", flags, std::slice::from_ref(service))
    }
    SetupAction::EnableService {
      service,
      user: true,
    } => CommandSpec::user(
      "systemctl",
      ["--user", "enable", "--now"],
      std::slice::from_ref(service),
    ),
    SetupAction::AddUserToGroup { group } => {
      CommandSpec::sudo("usermod", ["-aG"], &[group.clone(), ctx.user.clone()])
    }
    SetupAction::SetDefaultShell { shell } => {
      CommandSpec::sudo("usermod", ["--shell"], &[shell.clone(), ctx.user.clone()])
    }
  }
}

/// Checks that must pass right before running an action, because they can
/// only be true after the packages are installed.
pub fn precheck(action: &SetupAction, ctx: &Context) -> Result<(), String> {
  if let SetupAction::SetDefaultShell { shell } = action {
    let listed = fs::read_to_string(ctx.root.join("etc/shells"))
      .map(|text| text.lines().any(|line| line.trim() == shell))
      .unwrap_or(false);
    if !listed {
      return Err(format!(
        "{shell} is not listed in /etc/shells, so it can't be your login shell"
      ));
    }
  }
  Ok(())
}

/// A note shown after the action ran, if it needs one.
pub fn after_note(action: &SetupAction) -> Option<&'static str> {
  match action {
    SetupAction::AddUserToGroup { .. } | SetupAction::SetDefaultShell { .. } => {
      Some("log out and back in for this to take effect")
    }
    SetupAction::EnableService { .. } => None,
  }
}

fn service_enabled(service: &str, user: bool) -> ActionState {
  let mut command = Command::new("systemctl");
  if user {
    command.arg("--user");
  }
  let output = command
    .args(["is-enabled", service])
    .stdin(Stdio::null())
    .stderr(Stdio::null())
    .output();

  match output {
    Err(_) => ActionState::Unknown("systemctl was not found".into()),
    Ok(output) => {
      let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
      if matches!(
        answer.as_str(),
        "enabled" | "enabled-runtime" | "alias" | "static"
      ) {
        ActionState::Done
      } else {
        ActionState::Needed
      }
    }
  }
}

/// Members of a group listed in `<root>/etc/group` (`name:x:gid:a,b,c`).
pub fn group_members(root: &Path, group: &str) -> Option<Vec<String>> {
  let text = fs::read_to_string(root.join("etc/group")).ok()?;
  text.lines().find_map(|line| {
    let mut fields = line.split(':');
    (fields.next()? == group).then(|| {
      fields
        .nth(2)
        .unwrap_or("")
        .split(',')
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .collect()
    })
  })
}

/// The login shell of `user` in `<root>/etc/passwd` (7th field).
pub fn login_shell(root: &Path, user: &str) -> Option<String> {
  let text = fs::read_to_string(root.join("etc/passwd")).ok()?;
  text.lines().find_map(|line| {
    let fields: Vec<&str> = line.split(':').collect();
    (fields.first() == Some(&user) && fields.len() >= 7).then(|| fields[6].to_string())
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn ctx(root: &Path) -> Context {
    Context {
      root: root.to_path_buf(),
      user: "luky".into(),
      systemd: true,
    }
  }

  fn fake_etc() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("etc")).unwrap();
    fs::write(
      root.path().join("etc/group"),
      "wheel:x:998:luky\ndocker:x:970:\nvideo:x:985:other,luky\n",
    )
    .unwrap();
    fs::write(
      root.path().join("etc/passwd"),
      "root:x:0:0::/root:/bin/bash\nluky:x:1000:1000::/home/luky:/bin/bash\n",
    )
    .unwrap();
    fs::write(root.path().join("etc/shells"), "/bin/sh\n/bin/bash\n").unwrap();
    root
  }

  #[test]
  fn group_checks() {
    let root = fake_etc();
    let ctx = ctx(root.path());
    let add = |group: &str| SetupAction::AddUserToGroup {
      group: group.into(),
    };

    assert_eq!(check(&add("video"), &ctx), ActionState::Done);
    assert_eq!(check(&add("docker"), &ctx), ActionState::Needed);
    assert_eq!(check(&add("missing"), &ctx), ActionState::Needed);
    assert_eq!(
      command_for(&add("docker"), &ctx).to_string(),
      "sudo usermod -aG docker luky"
    );
  }

  #[test]
  fn shell_checks() {
    let root = fake_etc();
    let ctx = ctx(root.path());
    let bash = SetupAction::SetDefaultShell {
      shell: "/bin/bash".into(),
    };
    let zsh = SetupAction::SetDefaultShell {
      shell: "/usr/bin/zsh".into(),
    };

    assert_eq!(check(&bash, &ctx), ActionState::Done);
    assert_eq!(check(&zsh, &ctx), ActionState::Needed);
    assert!(precheck(&zsh, &ctx).unwrap_err().contains("/etc/shells"));
    assert!(precheck(&bash, &ctx).is_ok());
    assert_eq!(
      command_for(&zsh, &ctx).to_string(),
      "sudo usermod --shell /usr/bin/zsh luky"
    );
  }

  #[test]
  fn service_commands() {
    let root = fake_etc();
    let mut ctx = ctx(root.path());
    let system = SetupAction::EnableService {
      service: "sddm.service".into(),
      user: false,
    };
    let user = SetupAction::EnableService {
      service: "pipewire.socket".into(),
      user: true,
    };

    assert_eq!(
      command_for(&system, &ctx).to_string(),
      "sudo systemctl enable --now sddm.service"
    );
    assert_eq!(
      command_for(&user, &ctx).to_string(),
      "systemctl --user enable --now pipewire.socket"
    );

    ctx.systemd = false;
    assert_eq!(
      command_for(&system, &ctx).to_string(),
      "sudo systemctl enable sddm.service"
    );
  }
}
