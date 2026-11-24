# System detection and profile status

DotForge can look at the machine it runs on and tell you how far it is
from a profile. **This only reads. Nothing is installed or changed.**

```sh
dotforge system show               # what DotForge sees on this machine
dotforge profile status desktop    # what is installed, what is missing
```

## `dotforge system show`

```
This machine
────────────────────────────────
  system:              EndeavourOS
  distribution ID:     endeavouros
  based on:            arch
  DotForge family:     arch
  package manager:     pacman
  installed packages:  1042
  AUR helper:          paru
  repositories:        core, extra, multilib
  session:             wayland
  systemd:             yes
```

Where each line comes from:

| Line | Source |
|---|---|
| system, distribution ID, based on | `/etc/os-release` (`NAME`, `ID`, `ID_LIKE`) |
| DotForge family | `arch` when the ID or "based on" is `arch` (Arch, EndeavourOS, Manjaro, CachyOS, Garuda, ...). Other families are not supported yet. |
| package manager | The family's backend: `pacman` for Arch. |
| installed packages | pacman's local database, `/var/lib/pacman/local/` (read directly, no `pacman` command is run). |
| AUR helper | The first of `paru`, `yay` found on your `PATH`. |
| repositories | The `[name]` sections in `/etc/pacman.conf` that are not commented out. |
| session | `$XDG_SESSION_TYPE` (`wayland`, `x11`), empty outside a desktop session. |
| systemd | Whether `/run/systemd/system` exists. |

## `dotforge profile status <profile>`

1. The profile is checked first (like `profile validate`). A profile with
   errors is refused (exit code 4).
2. Every ID in `applications` and `rice` is looked up, and dependencies
   are added, in install order (see `dotforge catalog deps`).
3. For each entry, DotForge checks whether its packages for this
   machine's family are installed. A package also counts as installed
   when another installed package **provides** it, so `hyprland-git`
   satisfies `hyprland`.

Each entry gets one of three marks:

| Mark | Status | Meaning |
|---|---|---|
| `[ok  ]` | `installed` | All its packages are installed. |
| `[need]` | `missing` | At least one package still needs installing. |
| `[??  ]` | `unsupported` | DotForge can't check it here, e.g. not an Arch-family system, or the entry has no packages for this family. |

After the list you get a summary, the packages DotForge **would** install
(split into pacman and AUR), the setup steps from the catalog, and
warnings such as:

- the profile is for a different OS family than this machine;
- AUR packages are needed but neither paru nor yay is installed;
- an entry needs a repository that is not enabled (Steam needs
  `multilib`).

Setup steps (enabling services and so on) are only listed. Checking whether
they are already done is part of Milestone 5.

The exit code is 0 whenever the comparison worked, even if things are
missing. In JSON, use `data.satisfied` (`true` when nothing is missing or
unsupported) and `data.to_install`.

Not checked yet: the `system` section of a profile (audio, graphics and
hardware drivers), because those are not catalog entries yet.

## For tests: `DOTFORGE_SYSTEM_ROOT`

Setting `DOTFORGE_SYSTEM_ROOT=/some/folder` makes DotForge read
`/some/folder/etc/os-release`, `/some/folder/var/lib/pacman/local/`, and so
on instead of the real ones. The automated tests use it to fake Arch,
EndeavourOS, Ubuntu and Fedora machines.
