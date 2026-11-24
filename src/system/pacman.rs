//! Finding out which packages pacman has installed, and which AUR helper
//! is available.
//!
//! Instead of running `pacman -Q`, DotForge reads pacman's local database
//! directly: `/var/lib/pacman/local/<name>-<version>/desc`. That is the
//! same information, needs no extra program, and works with a test folder
//! in place of `/`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// AUR helpers DotForge can use, in order of preference.
pub const AUR_HELPERS: [&str; 2] = ["paru", "yay"];

#[derive(Debug, Clone, Default, Serialize)]
pub struct InstalledPackages {
  /// Names of installed packages.
  pub names: BTreeSet<String>,
  /// Names that installed packages "provide". For example `hyprland-git`
  /// provides `hyprland`, so Hyprland counts as installed.
  pub provides: BTreeSet<String>,
}

impl InstalledPackages {
  pub fn has(&self, package: &str) -> bool {
    self.names.contains(package) || self.provides.contains(package)
  }

  pub fn count(&self) -> usize {
    self.names.len()
  }
}

/// Reads `<root>/var/lib/pacman/local`. Returns `None` when there is no
/// pacman database (for example on a non-Arch system).
pub fn installed(root: &Path) -> Option<InstalledPackages> {
  let local = root.join("var/lib/pacman/local");
  let entries = fs::read_dir(&local).ok()?;
  let mut packages = InstalledPackages::default();

  for entry in entries.flatten() {
    let desc = entry.path().join("desc");
    let Ok(text) = fs::read_to_string(&desc) else {
      continue; // e.g. the ALPM_DB_VERSION file
    };
    parse_desc(&text, &mut packages);
  }

  Some(packages)
}

/// A `desc` file is a list of `%SECTION%` headers, each followed by values,
/// one per line, ending with a blank line.
fn parse_desc(text: &str, packages: &mut InstalledPackages) {
  let mut section = "";

  for line in text.lines() {
    let line = line.trim();

    if line.starts_with('%') && line.ends_with('%') {
      section = line;
      continue;
    }
    if line.is_empty() {
      section = "";
      continue;
    }

    match section {
      "%NAME%" => {
        packages.names.insert(line.to_string());
      }
      "%PROVIDES%" => {
        // `foo=1.2-1` or `libfoo.so=3-64`: keep only the name.
        let name = line.split(['=', '<', '>']).next().unwrap_or(line);
        packages.provides.insert(name.to_string());
      }
      _ => {}
    }
  }
}

/// The repositories enabled in `<root>/etc/pacman.conf`, such as `core`,
/// `extra` and `multilib`. A repository is enabled when its `[name]` line
/// is not commented out. `None` if there is no pacman.conf.
pub fn enabled_repositories(root: &Path) -> Option<BTreeSet<String>> {
  let text = fs::read_to_string(root.join("etc/pacman.conf")).ok()?;

  Some(
    text
      .lines()
      .map(str::trim)
      .filter_map(|line| line.strip_prefix('[')?.strip_suffix(']'))
      .filter(|name| *name != "options")
      .map(str::to_string)
      .collect(),
  )
}

/// Finds the first AUR helper on the PATH.
pub fn find_aur_helper() -> Option<String> {
  let path = std::env::var_os("PATH")?;

  AUR_HELPERS
    .iter()
    .find(|helper| std::env::split_paths(&path).any(|dir| is_executable(&dir.join(helper))))
    .map(|helper| helper.to_string())
}

fn is_executable(path: &PathBuf) -> bool {
  use std::os::unix::fs::PermissionsExt;

  fs::metadata(path)
    .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reads_names_and_provides() {
    let root = tempfile::tempdir().unwrap();
    let local = root.path().join("var/lib/pacman/local");

    let add = |dir: &str, desc: &str| {
      fs::create_dir_all(local.join(dir)).unwrap();
      fs::write(local.join(dir).join("desc"), desc).unwrap();
    };
    add("kitty-0.40.0-1", "%NAME%\nkitty\n\n%VERSION%\n0.40.0-1\n\n");
    add(
      "hyprland-git-0.50-1",
      "%NAME%\nhyprland-git\n\n%PROVIDES%\nhyprland=0.50\nlibhypr.so=1-64\n\n",
    );
    fs::write(local.join("ALPM_DB_VERSION"), "9\n").unwrap();

    let packages = installed(root.path()).unwrap();
    assert_eq!(packages.count(), 2);
    assert!(packages.has("kitty"));
    assert!(packages.has("hyprland"));
    assert!(packages.has("libhypr.so"));
    assert!(!packages.has("niri"));
  }

  #[test]
  fn no_database_means_none() {
    let root = tempfile::tempdir().unwrap();
    assert!(installed(root.path()).is_none());
  }

  #[test]
  fn repository_detection() {
    let root = tempfile::tempdir().unwrap();
    assert!(enabled_repositories(root.path()).is_none());

    fs::create_dir_all(root.path().join("etc")).unwrap();
    let conf = root.path().join("etc/pacman.conf");

    fs::write(
      &conf,
      "[options]\nColor\n[core]\nInclude = x\n#[multilib]\n#Include = y\n",
    )
    .unwrap();
    let repos = enabled_repositories(root.path()).unwrap();
    assert_eq!(repos.into_iter().collect::<Vec<_>>(), ["core"]);

    fs::write(&conf, "[core]\n  [multilib]\nInclude = y\n").unwrap();
    assert!(
      enabled_repositories(root.path())
        .unwrap()
        .contains("multilib")
    );
  }
}
