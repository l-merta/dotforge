# Catalog format

The catalog is DotForge's list of things it knows how to work with. Each
entry is one YAML file.

## Where entries come from

1. **Built-in:** the `catalog/` folder of this repository. It is copied
   into the program when it is compiled, so it always matches the
   program version and works from any folder.
2. **Your own:** `~/.config/dotforge/catalog/`. Entries here are added to
   the built-in ones. An entry with the **same ID** as a built-in entry
   **replaces** it (handy for trying a different package).

Check everything with `dotforge catalog validate`.

## Conventions

- **ID:** lowercase letters, digits and single dashes. Examples:
  `kitty`, `kde-plasma`, `xdg-desktop-portal-hyprland`. IDs must be unique.
- **File name:** `<id>.yml`, for example `hyprland.yml`.
- **Folder:** named after the category, in plural:

  | `category:` | Folder |
  |---|---|
  | `desktop` | `desktops/` |
  | `compositor` | `compositors/` |
  | `shell` | `shells/` |
  | `component` | `components/` |
  | `application` | `applications/` |

  For built-in entries a wrong folder is an error; for your own entries
  it is only a warning.
- **Dependencies:** a list of other catalog **IDs** (not package names).
  For now an unknown dependency is a warning, because the catalog is still
  small. Milestone 3 makes it an error.

## Example

```yaml
id: hyprland
name: Hyprland
description: Dynamic tiling Wayland compositor
category: compositor

platforms: [linux]
protocols: [wayland]

installation:
  arch:
    packages: [hyprland]

dependencies:
  - xdg-desktop-portal
  - xdg-desktop-portal-hyprland

capabilities: [compositor, window-management, keybindings, workspaces, animations]

configuration:
  directory: ~/.config/hypr

integration:
  themes: true
  wallpapers: true
  keybindings: true
```

## Fields

| Field | Required | Value |
|---|---|---|
| `id` | yes | See conventions above. |
| `name` | yes | Readable name. |
| `description` | yes | One sentence. |
| `category` | yes | `desktop`, `compositor`, `shell`, `component` or `application`. |
| `platforms` | no | `linux`. |
| `protocols` | no | `wayland`, `x11`. |
| `installation` | yes | Per OS family (`arch`, ...), a list of `packages`. Can be `{}` while unknown. |
| `dependencies` | no | Other catalog IDs. |
| `capabilities` | no | `compositor`, `window-management`, `keybindings`, `workspaces`, `animations`, `notifications`, `launcher`, `panel`, `wallpaper`, `theme`. |
| `configuration.directory` | no | Where its config lives, for example `~/.config/hypr`. |
| `integration` | no | `themes`, `wallpapers`, `keybindings`: `true`/`false`. |

Unknown field names are rejected so typos are caught. AUR packages and
typed setup actions are added in Milestone 3.
