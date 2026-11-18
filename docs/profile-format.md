# Profile format

A profile describes the machine you want. It lives in its own folder:

```
~/.config/dotforge/profiles/
└── desktop/
    ├── profile.yml   ← described on this page
    ├── config/       ← your real config files (used from Milestone 6)
    └── assets/       ← wallpapers, icons, fonts, ... (used from Milestone 6)
```

Check a profile with:

```sh
dotforge profile validate desktop          # by name
dotforge profile validate ./some/folder    # or by path
```

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

## What `validate` checks

- The YAML is readable, every required field is there and every fixed
  value (`device.type`, `device.os.family`) is allowed. Errors name the
  file, line and field, for example:
  `profile.yml:6:13: problem in field 'device.os.family': unknown variant `windows`, expected `arch``
- Every ID under `applications` and `rice` exists in the catalog and has
  the right category. Close misspellings get a "did you mean" tip.
- **Warning:** the same application listed twice.

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
