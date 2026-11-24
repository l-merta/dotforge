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

// ---------- settings and `profile edit` ----------

/// Writes a tiny "editor" script that appends `line` to the file it opens.
fn fake_editor(dir: &Path, line: &str) -> String {
  use std::os::unix::fs::PermissionsExt;

  let script = dir.join("fake-editor.sh");
  fs::write(&script, format!("#!/bin/sh\necho '{line}' >> \"$1\"\n")).unwrap();
  fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
  script.display().to_string()
}

fn set_editor(config: &Path, editor: &str) {
  fs::create_dir_all(config).unwrap();
  fs::write(config.join("config.yml"), format!("editor: {editor}\n")).unwrap();
}

#[test]
fn config_show_uses_nano_by_default() {
  let config = TempDir::new().unwrap();
  let value = json(&dotforge(config.path(), &["--json", "config", "show"]));

  assert_eq!(value["data"]["file_exists"], false);
  assert_eq!(value["data"]["settings"][0]["key"], "editor");
  assert_eq!(value["data"]["settings"][0]["value"], "nano");
  assert_eq!(value["data"]["settings"][0]["source"], "default");
}

#[test]
fn config_show_reads_the_settings_file() {
  let config = TempDir::new().unwrap();
  set_editor(config.path(), "code --wait");

  let value = json(&dotforge(config.path(), &["--json", "config", "show"]));
  assert_eq!(value["data"]["settings"][0]["value"], "code --wait");
  assert_eq!(value["data"]["settings"][0]["source"], "file");
}

#[test]
fn a_typo_in_the_settings_file_is_reported() {
  let config = TempDir::new().unwrap();
  fs::write(config.path().join("config.yml"), "editr: vim\n").unwrap();

  let output = dotforge(config.path(), &["--json", "config", "show"]);
  assert_eq!(output.status.code(), Some(4));
  assert!(
    json(&output)["error"]["message"]
      .as_str()
      .unwrap()
      .contains("editr")
  );
}

#[test]
fn profile_edit_opens_the_configured_editor_and_checks_the_result() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);
  let editor = fake_editor(config.path(), "description: edited by the fake editor");
  set_editor(config.path(), &editor);

  let output = dotforge(config.path(), &["--json", "profile", "edit", "desk"]);
  assert_eq!(
    output.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&output.stdout)
  );

  let value = json(&output);
  assert_eq!(value["data"]["validation"]["valid"], true);

  let contents = fs::read_to_string(config.path().join("profiles/desk/profile.yml")).unwrap();
  assert!(contents.ends_with("description: edited by the fake editor\n"));
}

#[test]
fn profile_edit_reports_a_profile_broken_by_the_edit() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);
  let editor = fake_editor(config.path(), "colour: red");
  set_editor(config.path(), &editor);

  let output = dotforge(config.path(), &["--json", "profile", "edit", "desk"]);
  assert_eq!(output.status.code(), Some(4));
  assert!(
    json(&output)["error"]["message"]
      .as_str()
      .unwrap()
      .contains("colour")
  );
}

#[test]
fn missing_editor_gives_a_hint() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);
  set_editor(config.path(), "no-such-editor-here");

  let output = dotforge(config.path(), &["--json", "profile", "edit", "desk"]);
  let value = json(&output);

  assert_eq!(output.status.code(), Some(1));
  assert_eq!(value["error"]["kind"], "editor");
  assert!(
    value["error"]["message"]
      .as_str()
      .unwrap()
      .contains("was not found")
  );
  assert!(
    value["error"]["hint"]
      .as_str()
      .unwrap()
      .contains("config.yml")
  );
}

#[test]
fn editor_that_fails_is_reported() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);
  set_editor(config.path(), "false");

  let output = dotforge(config.path(), &["profile", "edit", "desk"]);
  assert_eq!(output.status.code(), Some(1));
  assert!(String::from_utf8_lossy(&output.stderr).contains("exited with code 1"));
}

#[test]
fn profile_edit_unknown_profile() {
  let config = TempDir::new().unwrap();
  let output = dotforge(config.path(), &["profile", "edit", "ghost"]);
  assert_eq!(output.status.code(), Some(3));
}

/// Runs dotforge with a fake `nano` first on the PATH, so the default
/// editor can be tested without opening a real one.
fn dotforge_with_fake_nano(config: &Path, args: &[&str]) -> Output {
  use std::os::unix::fs::PermissionsExt;

  let bin = config.join("fake-bin");
  fs::create_dir_all(&bin).unwrap();
  let nano = bin.join("nano");
  fs::write(
    &nano,
    "#!/bin/sh\necho \"$1\" > \"$(dirname \"$0\")/opened\"\n",
  )
  .unwrap();
  fs::set_permissions(&nano, fs::Permissions::from_mode(0o755)).unwrap();

  let path = format!(
    "{}:{}",
    bin.display(),
    std::env::var("PATH").unwrap_or_default()
  );
  Command::new(env!("CARGO_BIN_EXE_dotforge"))
    .args(args)
    .env("DOTFORGE_CONFIG_DIR", config)
    .env("PATH", path)
    .current_dir(std::env::temp_dir())
    .output()
    .expect("failed to run dotforge")
}

#[test]
fn profile_edit_uses_nano_by_default() {
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);

  let output = dotforge_with_fake_nano(config.path(), &["profile", "edit", "desk"]);
  assert_eq!(
    output.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );

  let opened = fs::read_to_string(config.path().join("fake-bin/opened")).unwrap();
  assert!(
    opened.trim().ends_with("profiles/desk/profile.yml"),
    "{opened}"
  );
}

#[test]
fn config_edit_creates_the_file_from_the_template() {
  let config = TempDir::new().unwrap();
  let settings = config.path().join("config.yml");

  let output = dotforge_with_fake_nano(config.path(), &["--json", "config", "edit"]);
  assert_eq!(
    output.status.code(),
    Some(0),
    "{}",
    String::from_utf8_lossy(&output.stdout)
  );
  assert_eq!(json(&output)["data"]["created"], true);
  assert!(
    fs::read_to_string(&settings)
      .unwrap()
      .contains("editor: nano")
  );

  // Second time the file already exists.
  let again = dotforge_with_fake_nano(config.path(), &["--json", "config", "edit"]);
  assert_eq!(json(&again)["data"]["created"], false);
}

// ---------- Milestone 4: system detection and profile status ----------

/// Builds a fake system folder: an os-release file, a pacman database with
/// `installed` packages (each `name` or `name:provides`), and a pacman.conf.
fn fake_system(os_release: &str, installed: &[&str], pacman_conf: Option<&str>) -> TempDir {
  let root = TempDir::new().unwrap();
  fs::create_dir_all(root.path().join("etc")).unwrap();
  fs::write(root.path().join("etc/os-release"), os_release).unwrap();

  if let Some(conf) = pacman_conf {
    fs::write(root.path().join("etc/pacman.conf"), conf).unwrap();
    fs::create_dir_all(root.path().join("var/lib/pacman/local")).unwrap();
  }

  for package in installed {
    let (name, provides) = package.split_once(':').unwrap_or((package, ""));
    let dir = root
      .path()
      .join(format!("var/lib/pacman/local/{name}-1.0-1"));
    fs::create_dir_all(&dir).unwrap();
    let mut desc = format!("%NAME%\n{name}\n\n");
    if !provides.is_empty() {
      desc.push_str(&format!("%PROVIDES%\n{provides}=1.0\n\n"));
    }
    fs::write(dir.join("desc"), desc).unwrap();
  }

  root
}

const ARCH: &str = "NAME=\"Arch Linux\"\nID=arch\n";
const CONF: &str = "[options]\n[core]\n[extra]\n#[multilib]\n";

fn dotforge_on(system: &Path, config: &Path, path_extra: Option<&Path>, args: &[&str]) -> Output {
  let mut path = std::env::var("PATH").unwrap_or_default();
  if let Some(extra) = path_extra {
    path = format!("{}:{path}", extra.display());
  }
  Command::new(env!("CARGO_BIN_EXE_dotforge"))
    .args(args)
    .env("DOTFORGE_CONFIG_DIR", config)
    .env("DOTFORGE_SYSTEM_ROOT", system)
    .env("PATH", path)
    .current_dir(std::env::temp_dir())
    .output()
    .expect("failed to run dotforge")
}

#[test]
fn system_show_detects_an_arch_based_system() {
  use std::os::unix::fs::PermissionsExt;

  let system = fake_system(
    "NAME=\"EndeavourOS\"\nID=endeavouros\nID_LIKE=arch\n",
    &["kitty", "git"],
    Some(CONF),
  );
  let config = TempDir::new().unwrap();

  let bin = config.path().join("bin");
  fs::create_dir_all(&bin).unwrap();
  fs::write(bin.join("yay"), "#!/bin/sh\n").unwrap();
  fs::set_permissions(bin.join("yay"), fs::Permissions::from_mode(0o755)).unwrap();

  let output = dotforge_on(
    system.path(),
    config.path(),
    Some(&bin),
    &["--json", "system", "show"],
  );
  let data = &json(&output)["data"];

  assert_eq!(data["name"], "EndeavourOS");
  assert_eq!(data["family"], "arch");
  assert_eq!(data["supported"], true);
  assert_eq!(data["package_manager"], "pacman");
  assert_eq!(data["installed_packages"], 2);
  assert_eq!(data["aur_helper"], "yay");
  assert_eq!(data["repositories"], serde_json::json!(["core", "extra"]));
}

#[test]
fn system_show_on_an_unsupported_system() {
  let system = fake_system("NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n", &[], None);
  let config = TempDir::new().unwrap();

  let output = dotforge_on(
    system.path(),
    config.path(),
    None,
    &["--json", "system", "show"],
  );
  assert_eq!(output.status.code(), Some(0));

  let data = &json(&output)["data"];
  assert_eq!(data["supported"], false);
  assert!(data["problem"].as_str().unwrap().contains("Ubuntu"));
}

#[test]
fn profile_status_lists_what_is_missing() {
  let system = fake_system(ARCH, &["kitty", "xdg-desktop-portal"], Some(CONF));
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);

  let output = dotforge_on(
    system.path(),
    config.path(),
    None,
    &["--json", "profile", "status", "desk"],
  );
  assert_eq!(output.status.code(), Some(0));

  let data = &json(&output)["data"];
  assert_eq!(data["satisfied"], false);
  assert_eq!(data["counts"]["installed"], 2);
  assert_eq!(data["counts"]["missing"], 2);
  assert_eq!(
    data["to_install"]["packages"],
    serde_json::json!(["xdg-desktop-portal-hyprland", "hyprland"])
  );
}

#[test]
fn provided_packages_count_as_installed() {
  // hyprland-git "provides" hyprland, so Hyprland is already there.
  let system = fake_system(
    ARCH,
    &[
      "kitty",
      "xdg-desktop-portal",
      "xdg-desktop-portal-hyprland",
      "hyprland-git:hyprland",
    ],
    Some(CONF),
  );
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);

  let output = dotforge_on(
    system.path(),
    config.path(),
    None,
    &["profile", "status", "desk"],
  );
  let text = String::from_utf8_lossy(&output.stdout);
  assert!(
    text.contains("This machine already matches the profile."),
    "{text}"
  );
}

#[test]
fn profile_status_refuses_an_invalid_profile() {
  let system = fake_system(ARCH, &[], Some(CONF));
  let config = TempDir::new().unwrap();
  write_profile(
    config.path(),
    "desk",
    &PROFILE.replace("hyprland", "hyprlnd"),
  );

  let output = dotforge_on(
    system.path(),
    config.path(),
    None,
    &["profile", "status", "desk"],
  );
  assert_eq!(output.status.code(), Some(4));
  assert!(String::from_utf8_lossy(&output.stderr).contains("profile validate"));
}

#[test]
fn profile_status_on_an_unsupported_system() {
  let system = fake_system("NAME=\"Fedora Linux\"\nID=fedora\n", &[], None);
  let config = TempDir::new().unwrap();
  write_profile(config.path(), "desk", PROFILE);

  let output = dotforge_on(
    system.path(),
    config.path(),
    None,
    &["--json", "profile", "status", "desk"],
  );
  let data = &json(&output)["data"];

  assert_eq!(output.status.code(), Some(0));
  assert_eq!(data["counts"]["unsupported"], 4);
  assert_eq!(data["counts"]["missing"], 0);
}

#[test]
fn profile_status_warns_about_multilib_and_aur_helper() {
  let system = fake_system(ARCH, &[], Some(CONF));
  let config = TempDir::new().unwrap();
  write_profile(
    config.path(),
    "games",
    &PROFILE.replace(
      "terminal: kitty",
      "terminal: kitty\n  gaming: [steam, heroic]",
    ),
  );

  // An empty PATH extra and a clean PATH: no paru or yay anywhere.
  let output = Command::new(env!("CARGO_BIN_EXE_dotforge"))
    .args(["--json", "profile", "status", "games"])
    .env("DOTFORGE_CONFIG_DIR", config.path())
    .env("DOTFORGE_SYSTEM_ROOT", system.path())
    .env("PATH", "/nonexistent")
    .output()
    .unwrap();

  let notes = json(&output)["data"]["notes"].clone();
  let messages: Vec<&str> = notes
    .as_array()
    .unwrap()
    .iter()
    .map(|n| n["message"].as_str().unwrap())
    .collect();
  assert!(
    messages.iter().any(|m| m.contains("'multilib'")),
    "{messages:?}"
  );
  assert!(
    messages.iter().any(|m| m.contains("neither paru nor yay")),
    "{messages:?}"
  );
}

// ---------- Milestone 5: profile apply ----------
//
// These tests never touch the real system. Fake `sudo`, `pacman`,
// `systemctl` and `usermod` programs are put first on PATH. The fake
// pacman "installs" a package by writing its entry into the fake pacman
// database inside DOTFORGE_SYSTEM_ROOT, and the fake usermod edits the fake
// /etc/passwd. Real installs are tested in CI inside an Arch container.

struct ApplyEnv {
  system: TempDir,
  config: TempDir,
  state: TempDir,
  bin: TempDir,
}

const FAKE_SUDO: &str = "#!/bin/sh\nexec \"$@\"\n";

const FAKE_PACMAN: &str = r#"#!/bin/sh
echo "pacman $*" >> "$DOTFORGE_SYSTEM_ROOT/calls.log"
mode="$1"; shift
db="$DOTFORGE_SYSTEM_ROOT/var/lib/pacman/local"
for arg in "$@"; do
  case "$arg" in -*) continue ;; esac
  if [ -n "$FAIL_PACKAGE" ] && [ "$arg" = "$FAIL_PACKAGE" ]; then
    echo "error: target not found: $arg" >&2
    exit 1
  fi
  case "$mode" in
    -S) mkdir -p "$db/$arg-1.0-1" && printf '%%NAME%%\n%s\n\n' "$arg" > "$db/$arg-1.0-1/desc" ;;
    -Rs) rm -rf "$db/$arg-1.0-1" ;;
  esac
done
"#;

const FAKE_SYSTEMCTL: &str = "#!/bin/sh\necho \"systemctl $*\" >> \"$DOTFORGE_SYSTEM_ROOT/calls.log\"\n[ \"$1\" = is-enabled ] && { echo disabled; exit 1; }\nexit 0\n";

const FAKE_USERMOD: &str = r#"#!/bin/sh
echo "usermod $*" >> "$DOTFORGE_SYSTEM_ROOT/calls.log"
if [ "$1" = "--shell" ]; then
  sed -i "s#^\($3:.*:\)[^:]*\$#\1$2#" "$DOTFORGE_SYSTEM_ROOT/etc/passwd"
fi
"#;

impl ApplyEnv {
  fn new(installed: &[&str]) -> Self {
    use std::os::unix::fs::PermissionsExt;

    let system = fake_system(ARCH, installed, Some(CONF));
    let etc = system.path().join("etc");
    fs::write(
      etc.join("passwd"),
      "tester:x:1000:1000::/home/tester:/bin/bash\n",
    )
    .unwrap();
    fs::write(etc.join("group"), "wheel:x:998:tester\n").unwrap();
    fs::write(etc.join("shells"), "/bin/bash\n/usr/bin/zsh\n").unwrap();

    let bin = TempDir::new().unwrap();
    for (name, script) in [
      ("sudo", FAKE_SUDO),
      ("pacman", FAKE_PACMAN),
      ("systemctl", FAKE_SYSTEMCTL),
      ("usermod", FAKE_USERMOD),
    ] {
      let path = bin.path().join(name);
      fs::write(&path, script).unwrap();
      fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    ApplyEnv {
      system,
      config: TempDir::new().unwrap(),
      state: TempDir::new().unwrap(),
      bin,
    }
  }

  fn profile(&self, utilities: &str) {
    write_profile(
      self.config.path(),
      "box",
      &format!(
        "format_version: 1\nname: Box\ndevice:\n  type: desktop\n  os:\n    family: arch\n    distribution: arch\napplications:\n  utilities: [{utilities}]\n"
      ),
    );
  }

  fn run(&self, args: &[&str], stdin: &str) -> Output {
    self.run_with(args, stdin, &[])
  }

  fn run_with(&self, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Output {
    use std::io::Write;
    use std::process::Stdio;

    let path = format!(
      "{}:{}",
      self.bin.path().display(),
      std::env::var("PATH").unwrap_or_default()
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_dotforge"));
    command
      .args(args)
      .env("DOTFORGE_CONFIG_DIR", self.config.path())
      .env("DOTFORGE_SYSTEM_ROOT", self.system.path())
      .env("DOTFORGE_STATE_DIR", self.state.path())
      .env("DOTFORGE_ALLOW_ROOT", "1")
      .env("USER", "tester")
      .env("PATH", path)
      .current_dir(std::env::temp_dir())
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());
    for (key, value) in env {
      command.env(key, value);
    }

    let mut child = command.spawn().unwrap();
    child
      .stdin
      .take()
      .unwrap()
      .write_all(stdin.as_bytes())
      .unwrap();
    child.wait_with_output().unwrap()
  }

  fn installed(&self, package: &str) -> bool {
    self
      .system
      .path()
      .join(format!("var/lib/pacman/local/{package}-1.0-1"))
      .exists()
  }

  fn calls(&self) -> String {
    fs::read_to_string(self.system.path().join("calls.log")).unwrap_or_default()
  }
}

#[test]
fn apply_dry_run_shows_the_plan_and_changes_nothing() {
  let env = ApplyEnv::new(&["git"]);
  env.profile("git, htop");

  let output = env.run(&["profile", "apply", "box", "--dry-run"], "");
  let stdout = String::from_utf8_lossy(&output.stdout);

  assert_eq!(output.status.code(), Some(0), "{stdout}");
  assert!(stdout.contains("Already installed: git"), "{stdout}");
  assert!(
    stdout.contains("pacman -S --needed --noconfirm htop"),
    "{stdout}"
  );
  assert!(stdout.contains("Dry run"), "{stdout}");
  assert!(!env.installed("htop"));
  assert_eq!(env.calls(), "");
}

#[test]
fn apply_installs_packages_and_runs_setup_steps() {
  let env = ApplyEnv::new(&[]);
  env.profile("htop, zsh");

  let output = env.run(&["profile", "apply", "box", "--yes"], "");
  let stdout = String::from_utf8_lossy(&output.stdout);
  let stderr = String::from_utf8_lossy(&output.stderr);

  assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
  assert!(env.installed("htop") && env.installed("zsh"));
  assert!(env.calls().contains("usermod --shell /usr/bin/zsh tester"));
  let passwd = fs::read_to_string(env.system.path().join("etc/passwd")).unwrap();
  assert!(passwd.contains(":/usr/bin/zsh"), "{passwd}");
  assert!(stdout.contains("The profile was applied"), "{stdout}");

  // A log file was written.
  let logs: Vec<_> = fs::read_dir(env.state.path().join("logs"))
    .unwrap()
    .collect();
  assert_eq!(logs.len(), 1);
}

#[test]
fn apply_twice_has_nothing_to_do() {
  let env = ApplyEnv::new(&[]);
  env.profile("htop, zsh");
  assert_eq!(
    env
      .run(&["profile", "apply", "box", "--yes"], "")
      .status
      .code(),
    Some(0)
  );
  let calls_before = env.calls();

  let output = env.run(&["--json", "profile", "apply", "box", "--yes"], "");
  assert_eq!(output.status.code(), Some(0));
  assert_eq!(json(&output)["data"]["result"], "nothing_to_do");
  // No package was installed and no setup command ran the second time.
  assert_eq!(env.calls(), calls_before);
}

#[test]
fn answering_no_cancels_without_changes() {
  let env = ApplyEnv::new(&[]);
  env.profile("htop");

  let output = env.run(&["profile", "apply", "box"], "n\n");
  assert_eq!(output.status.code(), Some(7));
  assert!(String::from_utf8_lossy(&output.stdout).contains("Cancelled"));
  assert!(!env.installed("htop"));
}

#[test]
fn answering_yes_applies() {
  let env = ApplyEnv::new(&[]);
  env.profile("htop");

  let output = env.run(&["profile", "apply", "box"], "y\n");
  assert_eq!(output.status.code(), Some(0));
  assert!(env.installed("htop"));
}

#[test]
fn a_failed_install_exits_with_failure_and_skips_its_setup() {
  let env = ApplyEnv::new(&[]);
  env.profile("zsh");

  let output = env.run_with(
    &["--json", "profile", "apply", "box", "--yes"],
    "",
    &[("FAIL_PACKAGE", "zsh")],
  );
  assert_eq!(output.status.code(), Some(1));

  let report = &json(&output)["data"]["report"];
  assert_eq!(report["success"], false);
  assert!(report["counts"]["failed"].as_u64().unwrap() >= 1);
  assert!(report["counts"]["skipped"].as_u64().unwrap() >= 1);
  assert!(!env.calls().contains("usermod"));
}

#[test]
fn dropped_entries_are_kept_with_yes_and_removed_when_asked() {
  let env = ApplyEnv::new(&[]);
  env.profile("git, htop");
  assert_eq!(
    env
      .run(&["profile", "apply", "box", "--yes"], "")
      .status
      .code(),
    Some(0)
  );

  // htop is taken out of the profile.
  env.profile("git");

  // --yes never removes anything.
  env.run(&["profile", "apply", "box", "--yes"], "");
  assert!(env.installed("htop"));

  // Asked again, the answer "r" removes it.
  let output = env.run(&["profile", "apply", "box"], "r\n");
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert_eq!(output.status.code(), Some(0), "{stderr}");
  assert!(
    stderr.contains("'htop' is no longer in the profile"),
    "{stderr}"
  );
  assert!(!env.installed("htop"));
  assert!(env.installed("git"));

  // Now DotForge has forgotten it, so it doesn't ask again.
  let output = env.run(&["--json", "profile", "apply", "box"], "");
  assert_eq!(json(&output)["data"]["result"], "nothing_to_do");
}

#[test]
fn dropped_entries_can_be_ignored() {
  let env = ApplyEnv::new(&[]);
  env.profile("git, htop");
  env.run(&["profile", "apply", "box", "--yes"], "");
  env.profile("git");

  env.run(&["profile", "apply", "box"], "i\n");
  assert!(env.installed("htop"));

  // Ignored: DotForge stopped tracking it and won't ask again.
  let output = env.run(&["--json", "profile", "apply", "box"], "");
  assert_eq!(json(&output)["data"]["result"], "nothing_to_do");
}

#[test]
fn apply_refuses_an_invalid_profile() {
  let env = ApplyEnv::new(&[]);
  env.profile("not-a-real-app");

  let output = env.run(&["profile", "apply", "box", "--yes"], "");
  assert_eq!(output.status.code(), Some(4));
  assert!(String::from_utf8_lossy(&output.stderr).contains("profile validate"));
}

#[test]
fn apply_refuses_an_unsupported_system() {
  let env = ApplyEnv::new(&[]);
  env.profile("git");
  fs::write(
    env.system.path().join("etc/os-release"),
    "NAME=\"Ubuntu\"\nID=ubuntu\n",
  )
  .unwrap();

  let output = env.run(&["profile", "apply", "box", "--yes"], "");
  assert_eq!(output.status.code(), Some(4));
}

#[test]
fn apply_refuses_to_run_as_root() {
  // The test container itself runs as root, so this only checks the
  // refusal when we are root.
  if unsafe_is_root() {
    let env = ApplyEnv::new(&[]);
    env.profile("git");
    let output = env.run_with(
      &["profile", "apply", "box", "--yes"],
      "",
      &[("DOTFORGE_ALLOW_ROOT", "0")],
    );
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("as root"));
  }
}

fn unsafe_is_root() -> bool {
  Command::new("id")
    .arg("-u")
    .output()
    .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
    .unwrap_or(false)
}
