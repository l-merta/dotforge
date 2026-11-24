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

// ---------- Milestone 2: profile management ----------

#[test]
fn list_with_no_profiles_explains_what_to_do() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["profile", "list"]);

  assert_eq!(output.status.code(), Some(0));
  assert!(String::from_utf8_lossy(&output.stdout).contains("dotforge profile create"));
}

#[test]
fn create_then_list_show_and_validate() {
  let config = TempDir::new().unwrap();

  let output = dotforge(
    config.path(),
    &[
      "profile",
      "create",
      "work-laptop",
      "--device",
      "laptop",
      "--description",
      "Office",
    ],
  );
  assert_eq!(
    output.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );

  let folder = config.path().join("profiles/work-laptop");
  assert!(folder.join("profile.yml").is_file());
  assert!(folder.join("config").is_dir());
  assert!(folder.join("assets/wallpapers").is_dir());

  let list = json(&dotforge(config.path(), &["--json", "profile", "list"]));
  assert_eq!(list["data"]["profiles"][0]["name"], "work-laptop");
  assert_eq!(list["data"]["profiles"][0]["description"], "Office");
  assert_eq!(list["data"]["profiles"][0]["status"], "ok");

  let show = json(&dotforge(
    config.path(),
    &["--json", "profile", "show", "work-laptop"],
  ));
  assert_eq!(show["data"]["profile"]["device"]["type"], "laptop");

  let validate = dotforge(config.path(), &["profile", "validate", "work-laptop"]);
  assert_eq!(validate.status.code(), Some(0));
}

#[test]
fn create_refuses_to_overwrite() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);

  let output = dotforge(config.path(), &["--json", "profile", "create", "desk"]);
  assert_eq!(output.status.code(), Some(6));
  assert_eq!(json(&output)["error"]["kind"], "already_exists");

  // The existing profile is untouched.
  let contents = fs::read_to_string(config.path().join("profiles/desk/profile.yml")).unwrap();
  assert_eq!(contents, PROFILE);
}

#[test]
fn create_rejects_bad_names_with_a_suggestion() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["profile", "create", "My Desk"]);

  assert_eq!(output.status.code(), Some(4));
  assert!(String::from_utf8_lossy(&output.stderr).contains("'my-desk'"));
}

#[test]
fn list_survives_broken_and_missing_profiles() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "good", PROFILE);
  write_profile(config.path(), "broken", "name: [oops\n");
  fs::create_dir_all(config.path().join("profiles/empty")).unwrap();

  let output = dotforge(config.path(), &["--json", "profile", "list"]);
  assert_eq!(output.status.code(), Some(0));

  let value = json(&output);
  let status = |i: usize| value["data"]["profiles"][i]["status"].clone();
  assert_eq!(status(0), "unreadable"); // broken
  assert_eq!(status(1), "missing_file"); // empty
  assert_eq!(status(2), "ok"); // good
}

#[test]
fn list_warns_about_duplicate_display_names() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "one", PROFILE);
  write_profile(config.path(), "two", PROFILE);

  let value = json(&dotforge(config.path(), &["--json", "profile", "list"]));
  let message = value["data"]["issues"][0]["message"].as_str().unwrap();
  assert!(message.contains("'Desk'"), "{message}");
}

#[test]
fn show_explains_a_folder_without_profile_yml() {
  let config = TempDir::new().unwrap();
  fs::create_dir_all(config.path().join("profiles/empty")).unwrap();

  let output = dotforge(config.path(), &["profile", "show", "empty"]);
  assert_eq!(output.status.code(), Some(3));
  assert!(String::from_utf8_lossy(&output.stderr).contains("has no 'profile.yml'"));
}

#[test]
fn show_a_broken_profile_points_at_the_problem() {
  let config = TempDir::new().unwrap();
  write_profile(
    config.path(),
    "desk",
    &PROFILE.replace("type: desktop", "type: toaster"),
  );

  let output = dotforge(config.path(), &["--json", "profile", "show", "desk"]);
  assert_eq!(output.status.code(), Some(4));
  assert_eq!(json(&output)["error"]["field"], "device.type");
}

#[test]
fn show_by_path() {
  let config = TempDir::new().unwrap();
  let repo_profile = concat!(env!("CARGO_MANIFEST_DIR"), "/profiles/test");

  let value = json(&dotforge(
    config.path(),
    &["--json", "profile", "show", repo_profile],
  ));
  assert_eq!(value["data"]["profile"]["name"], "Test profile");
}

// ---------- Milestone 3: catalog ----------

fn write_user_entry(config: &Path, folder: &str, id: &str, contents: &str) {
  let dir = config.join("catalog").join(folder);
  fs::create_dir_all(&dir).unwrap();
  fs::write(dir.join(format!("{id}.yml")), contents).unwrap();
}

#[test]
fn builtin_catalog_validates_cleanly() {
  let config = TempDir::new().unwrap();
  let value = json(&dotforge(config.path(), &["--json", "catalog", "validate"]));

  assert_eq!(value["ok"], true);
  assert_eq!(value["data"]["errors"], 0);
  assert_eq!(value["data"]["warnings"], 0);
}

#[test]
fn repo_test_profile_now_validates() {
  let config = TempDir::new().unwrap();
  let repo_profile = concat!(env!("CARGO_MANIFEST_DIR"), "/profiles/test");
  let output = dotforge(config.path(), &["profile", "validate", repo_profile]);

  assert_eq!(
    output.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&output.stdout)
  );
}

#[test]
fn deps_lists_dependencies_first() {
  let config = TempDir::new().unwrap();
  let value = json(&dotforge(
    config.path(),
    &["--json", "catalog", "deps", "hyprland"],
  ));

  let order: Vec<&str> = value["data"]["order"]
    .as_array()
    .unwrap()
    .iter()
    .map(|step| step["id"].as_str().unwrap())
    .collect();
  assert_eq!(
    order,
    [
      "xdg-desktop-portal",
      "xdg-desktop-portal-hyprland",
      "hyprland"
    ]
  );
}

#[test]
fn deps_shows_aur_packages_and_setup_steps() {
  let config = TempDir::new().unwrap();

  let heroic = json(&dotforge(
    config.path(),
    &["--json", "catalog", "deps", "heroic"],
  ));
  assert_eq!(
    heroic["data"]["order"][0]["aur"][0],
    "heroic-games-launcher-bin"
  );

  let docker = json(&dotforge(
    config.path(),
    &["--json", "catalog", "show", "docker"],
  ));
  assert_eq!(docker["data"]["setup"][0]["action"], "enable_service");
  assert_eq!(docker["data"]["setup"][1]["group"], "docker");
}

#[test]
fn circular_dependencies_are_caught() {
  let config = TempDir::new().unwrap();
  let entry = |id: &str, dep: &str| {
    format!(
      "id: {id}\nname: {id}\ndescription: test\ncategory: application\nplatforms: [linux]\ninstallation:\n  arch:\n    packages: [{id}]\ndependencies: [{dep}]\n"
    )
  };
  write_user_entry(
    config.path(),
    "applications",
    "chicken",
    &entry("chicken", "egg"),
  );
  write_user_entry(
    config.path(),
    "applications",
    "egg",
    &entry("egg", "chicken"),
  );

  let validate = dotforge(config.path(), &["--json", "catalog", "validate"]);
  assert_eq!(validate.status.code(), Some(4));
  let message = json(&validate)["data"]["issues"][0]["message"]
    .as_str()
    .unwrap()
    .to_string();
  assert!(message.contains("circular dependency"), "{message}");

  let deps = dotforge(config.path(), &["catalog", "deps", "chicken"]);
  assert_eq!(deps.status.code(), Some(4));
  assert!(String::from_utf8_lossy(&deps.stderr).contains("chicken → egg → chicken"));
}

#[test]
fn free_form_commands_are_not_allowed() {
  let config = TempDir::new().unwrap();
  write_user_entry(
    config.path(),
    "applications",
    "sneaky",
    "id: sneaky\nname: Sneaky\ndescription: x\ncategory: application\ninstallation: {}\nsetup:\n  - action: run_command\n    command: echo hi\n",
  );

  let output = dotforge(config.path(), &["--json", "catalog", "list"]);
  assert_eq!(output.status.code(), Some(4));
  assert!(
    json(&output)["error"]["message"]
      .as_str()
      .unwrap()
      .contains("run_command")
  );
}

#[test]
fn profile_warns_about_wayland_parts_on_x11() {
  let config = TempDir::new().unwrap();
  write_user_entry(
    config.path(),
    "compositors",
    "oldwm",
    "id: oldwm\nname: Old WM\ndescription: X11 only\ncategory: compositor\nplatforms: [linux]\nprotocols: [x11]\ncapabilities: [compositor]\ninstallation:\n  arch:\n    packages: [oldwm]\n",
  );
  write_profile(
    config.path(),
    "retro",
    &PROFILE.replace(
      "compositor: hyprland",
      "compositor: oldwm\n  components:\n    taskbar: waybar",
    ),
  );

  let output = dotforge(config.path(), &["--json", "profile", "validate", "retro"]);
  let value = json(&output);

  assert_eq!(output.status.code(), Some(0)); // only a warning
  assert_eq!(value["data"]["warnings"], 1);
  assert!(
    value["data"]["issues"][0]["message"]
      .as_str()
      .unwrap()
      .contains("only works on Wayland")
  );
}
