# Catalog format

The catalog is DotForge's list of things it knows how to work with. Each
entry is one YAML file. To add one yourself, see
[adding-catalog-entries.md](adding-catalog-entries.md).

## Where entries come from

1. **Built-in:** the `catalog/` folder of this repository. It is copied
   into the program when it is compiled, so it always matches the
   program version and works from any folder.
2. **Your own:** `~/.config/dotforge/catalog/`. Entries here are added to
   the built-in ones. An entry with the **same ID** as a built-in entry
   **replaces** it (handy for trying a different package).

Useful commands:

```sh
dotforge catalog list [category]    # what is in the catalog
dotforge catalog show <id>          # one entry in detail
dotforge catalog deps <id>          # what it needs, in install order
dotforge catalog validate           # check every entry
```

## Example

```yaml
id: docker
name: Docker
description: Runs applications in containers
category: application

platforms: [linux]

installation:
  arch:
    packages: [docker]

setup:
  - action: enable_service
    service: docker.service
  - action: add_user_to_group
    group: docker
```

## Fields

Unknown field names are rejected, so typos are caught.

| Field | Required | Value |
|---|---|---|
| `id` | yes | Unique ID. See [Conventions](#conventions). |
| `name` | yes | Readable name. |
| `description` | yes | One sentence. Put it in `"quotes"` if it contains `: `. |
| `category` | yes | `desktop`, `compositor`, `shell`, `component` or `application`. |
| `platforms` | should | `[linux]`. |
| `protocols` | see below | `wayland`, `x11`. |
| `installation` | yes | Packages per OS family. See [Installation](#installation). |
| `dependencies` | no | Other catalog **IDs** this entry needs. See [Dependencies](#dependencies). |
| `capabilities` | see below | What the entry provides. See [Capabilities](#capabilities). |
| `configuration.directory` | no | Where its config lives. See [Configuration](#configuration). |
| `integration` | no | `themes`, `wallpapers`, `keybindings`: `true`/`false`. Which parts of a rice it can take part in. |
| `setup` | no | Typed steps after installing. See [Setup actions](#setup-actions). |

## Rules per category

| Category | Folder | Extra rules |
|---|---|---|
| `desktop` | `desktops/` | Must list `protocols`. Usually depends on its login screen (e.g. `sddm`). |
| `compositor` | `compositors/` | Must list `protocols`. Should have the `compositor` capability. |
| `shell` | `shells/` | Must list `protocols`. Lists the capabilities it covers (panel, launcher, ...). |
| `component` | `components/` | Should list at least one capability, because profiles pick components by job. |
| `application` | `applications/` | Nothing extra. Terminals should have the `terminal` capability. |

## Installation

Packages are listed per OS family. Only `arch` is supported for now, and
every built-in entry must have it.

```yaml
installation:
  arch:
    packages: [steam]                     # official repositories, installed with pacman
    aur: [heroic-games-launcher-bin]      # the AUR, installed with paru or yay
    repositories: [multilib]              # repositories `packages` need (optional)
```

- `packages` and `aur` can be used together. At least one must have a
  package in it.
- `repositories` lists pacman repositories that must be enabled for
  `packages`, e.g. `repositories: [multilib]` for Steam. `profile status`
  warns when one is not enabled in `/etc/pacman.conf`.
- `aur` is only allowed for `arch`.
- Package names use lowercase letters, digits and `@ . _ + -`.

## Dependencies

`dependencies` lists other catalog **IDs** (not package names):

```yaml
dependencies: [xdg-desktop-portal-hyprland]
```

DotForge resolves them depth-first: each entry's dependencies are placed
before the entry itself, and each entry appears only once. See it with
`dotforge catalog deps hyprland`:

```
1. xdg-desktop-portal
2. xdg-desktop-portal-hyprland
3. hyprland
```

`catalog validate` reports, as **errors**: a dependency that is not in
the catalog (with a "did you mean" tip), an entry that depends on itself,
and circular dependencies (`a → b → a`).

## Capabilities

`compositor`, `window-management`, `keybindings`, `workspaces`,
`animations`, `notifications`, `launcher`, `panel`, `wallpaper`, `theme`,
`terminal`, `display-manager`, `portal`, `lockscreen`.

Profiles use them to check that an entry fits its slot:

| Profile field | Expected capability |
|---|---|
| `applications.terminal` | `terminal` |
| `rice.components.taskbar`, `status` | `panel` |
| `rice.components.launcher` | `launcher` |
| `rice.components.notifications` | `notifications` |

A mismatch is a warning in `profile validate`, not an error.

## Configuration

```yaml
configuration:
  directory: ~/.config/hypr
```

`directory` is where the program reads its config. It must start with
`~/` or `/`.

How it will be deployed (Milestone 6): the matching files in a profile live
in `config/<entry id>/`, e.g. `profiles/desktop/config/hyprland/`.
DotForge links `~/.config/hypr` to that folder with a **symlink**, so the
profile stays the one place to edit them. Leave `configuration` out when a
program shares another entry's folder (Hyprlock uses `~/.config/hypr` too)
or keeps its config in a single file in your home folder (Zsh's
`~/.zshrc`).

## Setup actions

Some things are not done by installing a package. These are written as
**typed actions**, never as shell commands. That keeps the catalog
readable, safe, and lets DotForge check whether each step is already done
before doing it (Milestone 5). Any other `action` is rejected.

| Action | Fields | What it means |
|---|---|---|
| `enable_service` | `service` (a systemd unit: `.service`, `.socket`, `.timer` or `.path`), `user` (optional, `true` for a user service) | Start the service now and at every boot. |
| `add_user_to_group` | `group` | Add you to a Linux group, e.g. `docker` or `input`. |
| `set_default_shell` | `shell` (full path, e.g. `/usr/bin/zsh`) | Make it your login shell. |

```yaml
setup:
  - action: enable_service
    service: sddm.service
  - action: set_default_shell
    shell: /usr/bin/zsh
```

## Conventions

- **ID:** lowercase letters, digits and single dashes: `kitty`,
  `kde-plasma`, `xdg-desktop-portal-hyprland`. Unique across the catalog.
  Use the program's common name, not the package name
  (`telegram`, not `telegram-desktop`).
- **File name:** `<id>.yml`.
- **Folder:** the plural category (table above). For built-in entries a
  wrong folder is an error; for your own entries it is only a warning.
