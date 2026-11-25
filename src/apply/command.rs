//! Commands as data, so a plan can show exactly what will run before
//! anything runs.

use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandSpec {
  pub program: String,
  pub args: Vec<String>,
  /// Run through `sudo`, because it changes the system. Only these steps
  /// get administrator rights; DotForge itself never runs as root.
  pub sudo: bool,
  /// The folder to run in (the current folder if not set).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cwd: Option<String>,
}

impl CommandSpec {
  pub fn sudo<'a>(
    program: &str,
    flags: impl IntoIterator<Item = &'a str>,
    args: &[String],
  ) -> Self {
    Self::new(program, flags, args, true)
  }

  pub fn user<'a>(
    program: &str,
    flags: impl IntoIterator<Item = &'a str>,
    args: &[String],
  ) -> Self {
    Self::new(program, flags, args, false)
  }

  fn new<'a>(
    program: &str,
    flags: impl IntoIterator<Item = &'a str>,
    args: &[String],
    sudo: bool,
  ) -> Self {
    let mut all: Vec<String> = flags.into_iter().map(str::to_string).collect();
    all.extend(args.iter().cloned());
    Self {
      program: program.to_string(),
      args: all,
      sudo,
      cwd: None,
    }
  }

  pub fn in_dir(mut self, dir: &str) -> Self {
    self.cwd = Some(dir.to_string());
    self
  }

  /// The program and arguments actually executed (`sudo` first if needed).
  pub fn argv(&self) -> Vec<String> {
    let mut argv = Vec::new();
    if self.sudo {
      argv.push("sudo".to_string());
    }
    argv.push(self.program.clone());
    argv.extend(self.args.iter().cloned());
    argv
  }
}

impl fmt::Display for CommandSpec {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.argv().join(" "))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shows_sudo_only_when_needed() {
    let packages = vec!["kitty".to_string(), "git".to_string()];
    let install = CommandSpec::sudo("pacman", ["-S", "--needed"], &packages);
    assert_eq!(install.to_string(), "sudo pacman -S --needed kitty git");

    let aur = CommandSpec::user("paru", ["-S"], &packages);
    assert_eq!(aur.to_string(), "paru -S kitty git");
  }
}
