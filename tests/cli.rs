//! End-to-end tests: run the real `dotforge` program and check its output
//! and exit code. Each test uses its own temporary config folder, so your
//! real `~/.config/dotforge/` is never touched.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn dotforge(config: &Path, args: &[&str]) -> Output {
  Command::new(env!("CARGO_BIN_EXE_dotforge"))
    .args(args)
    .env("DOTFORGE_CONFIG_DIR", config)
    // Run from a folder that has no `catalog/` in it, to prove that the
    // built-in catalog does not depend on the current folder.
    .current_dir(std::env::temp_dir())
    .output()
    .expect("failed to run dotforge")
}

fn json(output: &Output) -> serde_json::Value {
  serde_json::from_slice(&output.stdout).expect("stdout should be JSON")
}

const PROFILE: &str = "format_version: 1
name: Desk
device:
  type: desktop
  os:
    family: arch
    distribution: arch
applications:
  terminal: kitty
rice:
  compositor: hyprland
";

fn write_profile(config: &Path, name: &str, contents: &str) {
  let dir = config.join("profiles").join(name);
  fs::create_dir_all(&dir).unwrap();
  fs::write(dir.join("profile.yml"), contents).unwrap();
}

#[test]
fn catalog_list_works_from_any_folder() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["catalog", "list"]);

  assert_eq!(output.status.code(), Some(0));
  assert!(String::from_utf8_lossy(&output.stdout).contains("hyprland - Hyprland"));
}

#[test]
fn json_output_has_the_standard_envelope() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["--json", "catalog", "list", "compositor"]);
  let value = json(&output);

  assert_eq!(value["ok"], true);
  assert_eq!(value["exit_code"], 0);
  assert!(
    value["data"]["entries"]
      .as_array()
      .unwrap()
      .iter()
      .all(|e| e["category"] == "compositor")
  );
}

#[test]
fn json_flag_works_after_the_subcommand_too() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["catalog", "show", "kitty", "--json"]);
  assert_eq!(json(&output)["data"]["id"], "kitty");
}

#[test]
fn unknown_entry_exits_with_not_found() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["--json", "catalog", "show", "nope"]);
  let value = json(&output);

  assert_eq!(output.status.code(), Some(3));
  assert_eq!(value["ok"], false);
  assert_eq!(value["error"]["kind"], "not_found");
}

#[test]
fn user_catalog_entry_overrides_builtin() {
  let config = TempDir::new().unwrap();
  let dir = config.path().join("catalog/applications");
  fs::create_dir_all(&dir).unwrap();
  fs::write(
    dir.join("kitty.yml"),
    "id: kitty\nname: My Kitty\ndescription: custom\ncategory: application\ninstallation:\n  arch:\n    packages: [kitty-git]\n",
  )
  .unwrap();

  let value = json(&dotforge(
    config.path(),
    &["--json", "catalog", "show", "kitty"],
  ));
  assert_eq!(value["data"]["name"], "My Kitty");
  assert_eq!(value["data"]["source"]["kind"], "user");
}

#[test]
fn broken_user_catalog_file_names_file_and_field() {
  let config = TempDir::new().unwrap();
  let dir = config.path().join("catalog/shells");
  fs::create_dir_all(&dir).unwrap();
  fs::write(
    dir.join("bad.yml"),
    "id: bad\nname: Bad\ndescription: x\ncategory: spaceship\ninstallation: {}\n",
  )
  .unwrap();

  let output = dotforge(config.path(), &["--json", "catalog", "list"]);
  let value = json(&output);

  assert_eq!(output.status.code(), Some(4));
  assert_eq!(value["error"]["kind"], "parse");
  assert_eq!(value["error"]["field"], "category");
  assert_eq!(value["error"]["line"], 4);
  assert!(
    value["error"]["file"]
      .as_str()
      .unwrap()
      .ends_with("bad.yml")
  );
}

#[test]
fn valid_profile_by_name() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);

  let output = dotforge(config.path(), &["profile", "validate", "desk"]);
  assert_eq!(
    output.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&output.stdout)
  );
}

#[test]
fn invalid_profile_lists_issues_and_exits_with_invalid() {
  let config = TempDir::new().unwrap();
  write_profile(
    config.path(),
    "desk",
    &PROFILE.replace("hyprland", "hyprlnd"),
  );

  let output = dotforge(config.path(), &["--json", "profile", "validate", "desk"]);
  let value = json(&output);

  assert_eq!(output.status.code(), Some(4));
  assert_eq!(value["ok"], false);
  assert_eq!(value["data"]["errors"], 1);
  assert_eq!(value["data"]["issues"][0]["location"], "rice.compositor");
}

#[test]
fn missing_profile_exits_with_not_found() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["profile", "validate", "ghost"]);

  assert_eq!(output.status.code(), Some(3));
  assert!(String::from_utf8_lossy(&output.stderr).contains("hint:"));
}

#[test]
fn unfinished_commands_say_so() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["profile", "apply", "desk"]);

  assert_eq!(output.status.code(), Some(5));
  assert!(String::from_utf8_lossy(&output.stderr).contains("Milestone 5"));
}

#[test]
fn bad_arguments_exit_with_usage() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["catalog", "fly"]);
  assert_eq!(output.status.code(), Some(2));
}
