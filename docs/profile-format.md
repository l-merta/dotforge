# Profile format

A profile describes the machine you want. It lives in its own folder:

```
~/.config/dotforge/profiles/
└── desktop/
    ├── profile.yml   ← described on this page
    ├── config/       ← your real config files (used from Milestone 6)
    └── assets/       ← wallpapers, icons, fonts, ... (used from Milestone 6)
```

## Working with profiles

```sh
dotforge profile create desktop            # new profile from a starter template
dotforge profile create work-laptop --device laptop --description "Office"
dotforge profile list                      # all your profiles
dotforge profile show desktop              # what a profile contains
dotforge profile edit desktop              # open it in your editor (nano by default)
dotforge profile validate desktop          # check it for mistakes
dotforge profile show ./some/folder        # any command also takes a path
```

`profile create` makes the folder, a `profile.yml` with only the required
fields filled in (the rest are commented-out examples), and the
`config/` and `assets/` folders (`wallpapers`, `icons`, `cursors`,
`fonts`). It never overwrites an existing profile.

`profile edit` opens `profile.yml` in the editor from your settings
(`editor:` in `~/.config/dotforge/config.yml`, `nano` if not set; see
[cli.md](cli.md#settings)). When the editor closes, DotForge checks the
profile straight away and shows any problems.

### Profile names

The **profile name** is the name of its folder, and it is what you type in
commands. The rules are the same as for catalog IDs: lowercase letters,
digits and single dashes, at most 64 characters (`desktop`,
`work-laptop`, `vm2`). `profile create` refuses other names and suggests a
fixed one (`"My Laptop"` → `my-laptop`).

The `name:` field *inside* `profile.yml` is a separate **display name**
and can be any text.

### Name or path?

- A plain name (`desktop`) is looked up in
  `~/.config/dotforge/profiles/desktop/profile.yml`.
- Anything with a `/` in it, or ending in `.yml`, is used as a path. A
  folder means `<folder>/profile.yml`.

### What `profile list` reports

Listing only reads each folder name and the `name:`/`description:` lines,
so one broken profile never stops the others from being listed. Each
profile gets a status:

| Status | Meaning |
|---|---|
| `ok` | `profile.yml` was found and its first fields could be read. Run `validate` for a full check. |
| `missing_file` | The folder has no `profile.yml`. |
| `unreadable` | `profile.yml` is not valid YAML. |

It also warns about folder names that break the naming rules, folders
that only differ in upper/lower case, and profiles that share the same
display name. Hidden folders (starting with `.`) are skipped.

## Full example

```yaml
format_version: 1
name: My desktop
description: Gaming PC in the living room

device:
  type: desktop
  os:
    family: arch
    distribution: arch

system:
  audio:
    backend: pipewire
  graphics:
    drivers: [mesa]
    api: [vulkan]
  hardware:
    drivers: [nvidia]

applications:
  terminal: kitty
  gaming: [steam]
  social: [discord]
  productivity: []
  utilities: []
  development: [git]
  system: [zsh]

rice:
  desktop: kde-plasma
  compositor: hyprland
  shell: noctalia
  components:
    taskbar: waybar
    status: waybar
    launcher: rofi
    notifications: dunst
```

## Fields

**Required** fields must be present. Everything else can be left out.
Unknown field names are rejected, so a typo such as `aplications:` is
reported instead of being silently ignored.

| Field | Required | Value |
|---|---|---|
| `format_version` | yes | Version of the profile *format*. Always `1` for now. See [Versioning](#versioning). |
| `name` | yes | A readable name. Must not be empty. |
| `description` | no | Free text. |
| `device.type` | yes | `desktop`, `laptop` or `virtual-machine`. |
| `device.os.family` | yes | The group of distributions that share a package manager. Only `arch` for now. |
| `device.os.distribution` | yes | The specific distribution, for example `arch` or `endeavouros`. Must not be empty. |
| `system.audio.backend` | no | For example `pipewire`. Not checked yet. |
| `system.graphics.drivers` | no | List, for example `[mesa]`. Not checked yet. |
| `system.graphics.api` | no | List, for example `[vulkan]`. Not checked yet. |
| `system.hardware.drivers` | no | List, for example `[nvidia]`. Not checked yet. |
| `applications.terminal` | no | One catalog ID of category `application`. |
| `applications.gaming`, `social`, `productivity`, `utilities`, `development`, `system` | no | Lists of catalog IDs of category `application`. The groups are only for your own overview. |
| `rice.desktop` | no | Catalog ID of category `desktop`. |
| `rice.compositor` | no | Catalog ID of category `compositor`. |
| `rice.shell` | no | Catalog ID of category `shell`. |
| `rice.components.taskbar`, `status`, `launcher`, `notifications` | no | Catalog ID of category `component` or `shell` (a desktop shell can provide these itself). |
| `scripts` | no | List of your own scripts, relative to the profile folder, for example `[scripts/tweaks.sh]`. An escape hatch: see [Custom scripts](apply.md#custom-scripts). |

## What `validate` checks

- The YAML is readable, every required field is there and every fixed
  value (`device.type`, `device.os.family`) is allowed. Errors name the
  file, line and field, for example:
  `profile.yml:6:13: problem in field 'device.os.family': unknown variant `windows`, expected `arch``
- Every ID under `applications` and `rice` exists in the catalog and has
  the right category. Close misspellings get a "did you mean" tip.
- **Warning:** the same application listed twice.
- **Warning:** an entry in a slot it is not made for, e.g. a notification
  daemon as the launcher. This uses the entry's capabilities (see
  [catalog-format.md](catalog-format.md#capabilities)).
- **Warning:** a shell or component that only works on Wayland while the
  compositor (or desktop) only runs on X11, or the other way round.

- The `config/` and `assets/` folders: folder names in `config/` are
  catalog IDs of entries in the profile, and no two things want the same
  place. See [config-and-assets.md](config-and-assets.md).
- Every script in `scripts` is a relative path inside the profile folder
  (no `/...`, `~/...` or `..`) and the file exists.

The `system` section is not checked against the catalog yet; that comes
when drivers and audio stacks get catalog entries.

## Versioning

`format_version` says which version of this format the file is written
in. It is *not* a version number for your own setup.

- DotForge reads `format_version` first, before anything else.
- A profile with a **newer** version than DotForge understands gets a clear
  "please update DotForge" error.
- The number only goes up when a change would make old profiles be read
  wrongly (a field renamed, removed or changing meaning). Adding a new
  optional field does not change the version.
- When the version goes up, DotForge will explain how to update older
  profiles.
