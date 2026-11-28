//! Config files and assets: linking them from the profile folder into your
//! home folder.
//!
//! The files in the profile are the originals. Their destinations become
//! symlinks pointing back at them, so the profile folder is the one place
//! where every config is edited.
//!
//! ```text
//! profiles/desktop/
//! ├── config/
//! │   ├── hyprland/     -> ~/.config/hypr        (the catalog says where)
//! │   ├── kitty/        -> ~/.config/kitty
//! │   └── home/         -> mirrors ~, file by file (e.g. home/.zshrc -> ~/.zshrc)
//! └── assets/
//!     ├── wallpapers/   -> ~/.local/share/dotforge/wallpapers
//!     ├── fonts/        -> ~/.local/share/fonts/dotforge
//!     ├── icons/<name>/ -> ~/.local/share/icons/<name>
//!     └── cursors/<name>/ -> ~/.local/share/icons/<name>
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::state::ProfileState;
use crate::catalog::catalog::Catalog;
use crate::validation::Issue;

/// One link DotForge wants: `target` (in your home) pointing at `source`
/// (in the profile).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Wanted {
  /// `config`, `home` or `asset`.
  pub kind: &'static str,
  /// What it is, e.g. `hyprland`, `.zshrc` or `wallpapers`.
  pub label: String,
  pub source: PathBuf,
  pub target: PathBuf,
}

/// Works out every link the profile folder asks for, and any problems
/// with it. Only reads the profile folder; doesn't look at your home.
///
/// `wanted_ids` are the catalog IDs the profile installs (with their
/// dependencies): only their config folders are linked.
pub fn collect(
  profile_dir: &Path,
  wanted_ids: &BTreeSet<String>,
  catalog: &Catalog,
  home: &Path,
) -> (Vec<Wanted>, Vec<Issue>) {
  let mut links = Vec::new();
  let mut issues = Vec::new();

  // config/
  for (name, path) in children(&profile_dir.join("config")) {
    let location = format!("config/{name}");
    if !path.is_dir() {
      issues.push(Issue::warning(
        location,
        "files directly in config/ are ignored; put them in config/<catalog id>/ or config/home/",
      ));
      continue;
    }

    // An empty folder (like the ones `profile create` makes) has nothing
    // to link. Linking it would hide your real config behind an empty one.
    if is_empty_dir(&path) {
      continue;
    }

    if name == "home" {
      for file in files_under(&path) {
        let relative = file.strip_prefix(&path).unwrap_or(&file).to_path_buf();
        links.push(Wanted {
          kind: "home",
          label: format!("~/{}", relative.display()),
          source: file.clone(),
          target: home.join(&relative),
        });
      }
      continue;
    }

    let Some(entry) = catalog.get(&name) else {
      let similar = catalog.similar_ids(&name);
      let tip = if similar.is_empty() {
        "; use a catalog ID, or config/home/ for files that belong to no catalog entry".to_string()
      } else {
        format!("; did you mean '{}'?", similar.join("', '"))
      };
      issues.push(Issue::error(
        location,
        format!("there is no catalog entry '{name}'{tip}"),
      ));
      continue;
    };
    let Some(configuration) = &entry.configuration else {
      issues.push(Issue::error(
        location,
        format!(
          "the catalog doesn't know where '{name}' keeps its config; put these files in config/home/ instead"
        ),
      ));
      continue;
    };
    if !wanted_ids.contains(&name) {
      issues.push(Issue::warning(
        location,
        format!("'{name}' is not in the profile, so this config is not linked"),
      ));
      continue;
    }

    links.push(Wanted {
      kind: "config",
      label: name.clone(),
      source: path.clone(),
      target: expand_home(&configuration.directory, home),
    });
  }

  // assets/
  for (name, path) in children(&profile_dir.join("assets")) {
    let location = format!("assets/{name}");
    if !path.is_dir() {
      issues.push(Issue::warning(
        location,
        "files directly in assets/ are ignored",
      ));
      continue;
    }
    if is_empty_dir(&path) {
      continue;
    }
    let share = home.join(".local/share");
    match name.as_str() {
      "wallpapers" => links.push(asset(
        "wallpapers",
        &path,
        share.join("dotforge/wallpapers"),
      )),
      "fonts" => links.push(asset("fonts", &path, share.join("fonts/dotforge"))),
      "icons" | "cursors" => {
        for (theme, theme_path) in children(&path) {
          if theme_path.is_dir() {
            if is_empty_dir(&theme_path) {
              continue;
            }
            links.push(asset(
              &format!("{name}/{theme}"),
              &theme_path,
              share.join("icons").join(&theme),
            ));
          } else {
            issues.push(Issue::warning(
              format!("{location}/{theme}"),
              format!("each theme in assets/{name}/ must be a folder"),
            ));
          }
        }
      }
      _ => issues.push(Issue::warning(
        location,
        "unknown kind of asset; use wallpapers, fonts, icons or cursors",
      )),
    }
  }

  issues.extend(conflicts(&links));
  (links, issues)
}

/// The catalog IDs a profile installs, with everything they depend on.
pub fn profile_ids(
  profile: &crate::profile::model::Profile,
  catalog: &Catalog,
) -> BTreeSet<String> {
  let wanted = crate::system::compare::wanted_ids(profile);
  let roots: Vec<&str> = wanted.iter().map(|(_, id)| id.as_str()).collect();
  match crate::catalog::resolve::install_order(catalog, &roots) {
    Ok(order) => order.into_iter().collect(),
    Err(_) => roots.iter().map(|id| id.to_string()).collect(),
  }
}

/// Only the problems: what `profile validate` reports about config/ and
/// assets/.
pub fn check(
  profile: &crate::profile::model::Profile,
  profile_dir: &Path,
  catalog: &Catalog,
) -> Vec<Issue> {
  let home = crate::paths::home_dir().unwrap_or_else(|_| PathBuf::from("/home/you"));
  collect(profile_dir, &profile_ids(profile, catalog), catalog, &home).1
}

fn asset(label: &str, source: &Path, target: PathBuf) -> Wanted {
  Wanted {
    kind: "asset",
    label: label.to_string(),
    source: source.to_path_buf(),
    target,
  }
}

/// Two links for the same place, or one inside the other (for example
/// `config/home/.config/hypr/x.conf` and `config/hyprland/`).
fn conflicts(links: &[Wanted]) -> Vec<Issue> {
  let mut issues = Vec::new();
  for (index, a) in links.iter().enumerate() {
    for b in &links[index + 1..] {
      let clash =
        a.target == b.target || a.target.starts_with(&b.target) || b.target.starts_with(&a.target);
      if clash {
        issues.push(Issue::error(
          "config",
          format!(
            "'{}' and '{}' both want {}; keep only one of them",
            a.label,
            b.label,
            a.target
              .display()
              .to_string()
              .min(b.target.display().to_string())
          ),
        ));
      }
    }
  }
  issues
}

/// What is at a link's destination right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TargetState {
  /// Already a symlink to the profile: nothing to do.
  Linked,
  /// Nothing there yet.
  Create,
  /// Something else is there. It is moved to `backup` first.
  Replace { found: String, backup: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkStep {
  #[serde(flatten)]
  pub link: Wanted,
  #[serde(flatten)]
  pub state: TargetState,
}

impl LinkStep {
  pub fn needs_change(&self) -> bool {
    self.state != TargetState::Linked
  }
}

/// A link DotForge made earlier that the profile no longer has.
#[derive(Debug, Clone, Serialize)]
pub struct UnlinkStep {
  pub target: PathBuf,
  pub source: PathBuf,
}

/// Looks at your home folder and decides what each link needs.
/// `backup_dir` is where anything in the way would be moved.
pub fn plan(
  links: Vec<Wanted>,
  home: &Path,
  backup_dir: &Path,
  previous: Option<&ProfileState>,
) -> (Vec<LinkStep>, Vec<UnlinkStep>) {
  let steps: Vec<LinkStep> = links
    .into_iter()
    .map(|link| {
      let state = target_state(&link, home, backup_dir);
      LinkStep { link, state }
    })
    .collect();

  let targets: BTreeSet<&Path> = steps.iter().map(|s| s.link.target.as_path()).collect();
  let unlinks = previous
    .map(|p| &p.links)
    .into_iter()
    .flatten()
    .filter(|(target, _)| !targets.contains(Path::new(target)))
    .filter(|(target, source)| points_to(Path::new(target), Path::new(source)))
    .map(|(target, source)| UnlinkStep {
      target: PathBuf::from(target),
      source: PathBuf::from(source),
    })
    .collect();

  (steps, unlinks)
}

fn target_state(link: &Wanted, home: &Path, backup_dir: &Path) -> TargetState {
  let Ok(meta) = fs::symlink_metadata(&link.target) else {
    return TargetState::Create;
  };

  if meta.file_type().is_symlink() && points_to(&link.target, &link.source) {
    return TargetState::Linked;
  }

  let found = if meta.file_type().is_symlink() {
    match fs::read_link(&link.target) {
      Ok(to) => format!("a link to {}", to.display()),
      Err(_) => "a link".into(),
    }
  } else if meta.is_dir() {
    "a folder".into()
  } else {
    "a file".into()
  };

  let relative = link
    .target
    .strip_prefix(home)
    .unwrap_or_else(|_| link.target.strip_prefix("/").unwrap_or(&link.target));
  TargetState::Replace {
    found,
    backup: backup_dir.join(relative).display().to_string(),
  }
}

fn points_to(link: &Path, source: &Path) -> bool {
  fs::read_link(link).is_ok_and(|to| to == source)
}

/// Makes one link: creates the parent folders, moves anything in the way
/// to its backup, then creates the symlink.
pub fn make(step: &LinkStep) -> Result<(), String> {
  let target = &step.link.target;
  if let Some(parent) = target.parent() {
    fs::create_dir_all(parent).map_err(|e| format!("can't create {}: {e}", parent.display()))?;
  }

  if let TargetState::Replace { backup, .. } = &step.state {
    let backup = Path::new(backup);
    if let Some(parent) = backup.parent() {
      fs::create_dir_all(parent).map_err(|e| format!("can't create {}: {e}", parent.display()))?;
    }
    fs::rename(target, backup).map_err(|e| {
      format!(
        "can't move {} to {}: {e}",
        target.display(),
        backup.display()
      )
    })?;
  }

  std::os::unix::fs::symlink(&step.link.source, target)
    .map_err(|e| format!("can't create the link {}: {e}", target.display()))
}

/// Removes a link DotForge made (only the link; the files stay in the
/// profile folder).
pub fn unmake(step: &UnlinkStep) -> Result<(), String> {
  if !points_to(&step.target, &step.source) {
    return Ok(());
  }
  fs::remove_file(&step.target).map_err(|e| format!("can't remove {}: {e}", step.target.display()))
}

/// Shows `/home/you/.config/hypr` as `~/.config/hypr`.
pub fn tilde(path: &Path, home: &Path) -> String {
  match path.strip_prefix(home) {
    Ok(rest) => format!("~/{}", rest.display()),
    Err(_) => path.display().to_string(),
  }
}

fn expand_home(directory: &str, home: &Path) -> PathBuf {
  match directory.strip_prefix("~/") {
    Some(rest) => home.join(rest),
    None => PathBuf::from(directory),
  }
}

/// Folder contents as (name, path), sorted; empty if it doesn't exist.
fn children(dir: &Path) -> Vec<(String, PathBuf)> {
  let mut items: Vec<(String, PathBuf)> = fs::read_dir(dir)
    .into_iter()
    .flatten()
    .flatten()
    .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
    .collect();
  items.sort();
  items
}

fn is_empty_dir(dir: &Path) -> bool {
  fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}

/// Every file (and symlink) below `dir`, sorted.
fn files_under(dir: &Path) -> Vec<PathBuf> {
  let mut files: Vec<PathBuf> = walkdir::WalkDir::new(dir)
    .follow_links(false)
    .into_iter()
    .flatten()
    .filter(|e| !e.file_type().is_dir())
    .map(|e| e.into_path())
    .collect();
  files.sort();
  files
}

/// Updates what DotForge remembers: every link that is now in place.
pub fn remember(state: &mut ProfileState, made: &BTreeMap<PathBuf, PathBuf>, removed: &[PathBuf]) {
  for (target, source) in made {
    state
      .links
      .insert(target.display().to_string(), source.display().to_string());
  }
  for target in removed {
    state.links.remove(&target.display().to_string());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn catalog() -> Catalog {
    Catalog::load_with_user_dir(Path::new("/nonexistent")).unwrap()
  }

  fn ids(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
  }

  #[test]
  fn maps_config_home_and_assets() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    for folder in [
      "config/hyprland",
      "config/home/.config/git",
      "assets/wallpapers",
      "assets/icons/Papirus",
    ] {
      fs::create_dir_all(p.join(folder)).unwrap();
      fs::write(p.join(folder).join("some.file"), "").unwrap();
    }
    fs::write(p.join("config/hyprland/hyprland.conf"), "").unwrap();
    fs::write(p.join("config/home/.zshrc"), "").unwrap();
    fs::write(p.join("config/home/.config/git/config"), "").unwrap();

    let home = Path::new("/home/me");
    let (links, issues) = collect(p, &ids(&["hyprland"]), &catalog(), home);
    assert!(issues.is_empty(), "{issues:?}");

    let pairs: Vec<(String, String)> = links
      .iter()
      .map(|l| (l.label.clone(), tilde(&l.target, home)))
      .collect();
    assert_eq!(
      pairs,
      [
        ("~/.config/git/config".into(), "~/.config/git/config".into()),
        (
          "~/.config/git/some.file".into(),
          "~/.config/git/some.file".into()
        ),
        ("~/.zshrc".into(), "~/.zshrc".into()),
        ("hyprland".into(), "~/.config/hypr".into()),
        (
          "icons/Papirus".into(),
          "~/.local/share/icons/Papirus".into()
        ),
        (
          "wallpapers".into(),
          "~/.local/share/dotforge/wallpapers".into()
        ),
      ]
    );
  }

  #[test]
  fn empty_folders_are_not_linked() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    for folder in [
      "config/kitty",
      "config/home",
      "assets/fonts",
      "assets/icons/Empty",
    ] {
      fs::create_dir_all(p.join(folder)).unwrap();
    }
    let (links, issues) = collect(p, &ids(&["kitty"]), &catalog(), Path::new("/home/me"));
    assert!(links.is_empty(), "{links:?}");
    assert!(issues.is_empty(), "{issues:?}");
  }

  #[test]
  fn reports_unknown_unused_and_conflicting_configs() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    for folder in [
      "config/hyprlnd",
      "config/kitty",
      "config/git",
      "config/hyprland",
      "config/home/.config/hypr",
    ] {
      fs::create_dir_all(p.join(folder)).unwrap();
      fs::write(p.join(folder).join("some.file"), "").unwrap();
    }
    fs::write(p.join("config/home/.config/hypr/extra.conf"), "").unwrap();

    let (_, issues) = collect(
      p,
      &ids(&["hyprland", "git"]),
      &catalog(),
      Path::new("/home/me"),
    );
    let text: Vec<String> = issues
      .iter()
      .map(|i| format!("{}: {}", i.location, i.message))
      .collect();
    let has = |needle: &str| text.iter().any(|t| t.contains(needle));

    assert!(
      has("config/hyprlnd: there is no catalog entry 'hyprlnd'; did you mean 'hyprland'?"),
      "{text:?}"
    );
    assert!(
      has("config/kitty: 'kitty' is not in the profile"),
      "{text:?}"
    );
    assert!(
      has("config/git: the catalog doesn't know where 'git' keeps its config"),
      "{text:?}"
    );
    assert!(has("both want"), "{text:?}");
  }

  #[test]
  fn plans_create_linked_and_replace() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let profile = dir.path().join("profile");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(profile.join("a")).unwrap();

    let wanted = |name: &str| Wanted {
      kind: "home",
      label: name.into(),
      source: profile.join("a"),
      target: home.join(name),
    };
    // already linked
    std::os::unix::fs::symlink(profile.join("a"), home.join("linked")).unwrap();
    // something in the way
    fs::write(home.join("taken"), "mine").unwrap();

    let backups = dir.path().join("backups");
    let (steps, _) = plan(
      vec![wanted("new"), wanted("linked"), wanted("taken")],
      &home,
      &backups,
      None,
    );
    assert_eq!(steps[0].state, TargetState::Create);
    assert_eq!(steps[1].state, TargetState::Linked);
    assert_eq!(
      steps[2].state,
      TargetState::Replace {
        found: "a file".into(),
        backup: backups.join("taken").display().to_string()
      }
    );

    for step in &steps {
      if step.needs_change() {
        make(step).unwrap();
      }
    }
    assert!(points_to(&home.join("new"), &profile.join("a")));
    assert!(points_to(&home.join("taken"), &profile.join("a")));
    assert_eq!(fs::read_to_string(backups.join("taken")).unwrap(), "mine");
  }

  #[test]
  fn unlinks_only_its_own_links() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let ours = dir.path().join("ours");
    let replaced = dir.path().join("replaced");
    std::os::unix::fs::symlink(&source, &ours).unwrap();
    fs::write(&replaced, "user put a real file back").unwrap();

    let mut previous = ProfileState::default();
    for target in [&ours, &replaced] {
      previous
        .links
        .insert(target.display().to_string(), source.display().to_string());
    }

    let (_, unlinks) = plan(vec![], dir.path(), dir.path(), Some(&previous));
    assert_eq!(unlinks.len(), 1);
    assert_eq!(unlinks[0].target, ours);
    unmake(&unlinks[0]).unwrap();
    assert!(!ours.exists());
    assert!(replaced.exists());
  }
}
