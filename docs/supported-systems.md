# Supported systems and known limitations

## Supported

DotForge's first release supports **one OS family: Arch**.

| Distribution | Status |
|---|---|
| Arch Linux | Supported and tested. |
| Arch-based distributions (EndeavourOS, Manjaro, Garuda, CachyOS, ...) | Detected as the Arch family (through `ID_LIKE=arch` in `/etc/os-release`) and should work, but are not tested yet. |
| Everything else (Debian, Ubuntu, Fedora, openSUSE, ...) | Not supported yet. `system show` says so, `profile status` marks every line as "can't check", and `profile apply` refuses to run (exit code 4). |

Run `dotforge system show` to see what DotForge detects on your machine.

You also need:

- `sudo`, set up for your user. DotForge runs as you and uses `sudo` only
  for the steps that change the system.
- `paru` or `yay` if your profile has AUR packages. DotForge doesn't
  install an AUR helper for you.
- systemd, for `enable_service` setup steps. Without it (for example in a
  container) those steps are skipped, with a note.

## How it is tested

| Where | What | When |
|---|---|---|
| Unit and end-to-end tests | Every command, using fake `pacman`, `sudo`, `systemctl` and `usermod` programs and a fake home folder, so nothing real is changed. | Every pull request. |
| A clean `archlinux:latest` container on GitHub Actions | A real install: dry run, apply, apply again, removing an item, a custom script, config links and backups, and refusing to run as root (`ci/arch-apply-test.sh`). | Every pull request. |
| A second clean container | The complete workflow a new user goes through: create a profile, add your own catalog entry, check status, dry run, apply a Hyprland desktop with apps, Docker and an AUR package (through a real `paru`), status afterwards, apply again. Then the ways it can go wrong: an invalid profile, a package that doesn't exist and a fixed re-run, and a busy pacman (`ci/arch-workflow-test.sh`). | Every pull request. |
| A clean Arch virtual machine | The hand tests in [manual-tests.md](manual-tests.md), including a full desktop with systemd services and logging in to the result. | Before each release. |

## Known limitations

- **No rollback.** DotForge doesn't undo an apply. Each step is checked
  before it runs, and files in the way of a config link are backed up,
  but removing what was installed means taking it out of the profile and
  choosing **remove**.
- **Interrupting an apply** (Ctrl+C, or a power cut) stops the current
  command. Running `apply` again is safe and continues where it stopped,
  because finished steps are detected as done. If pacman itself was
  interrupted it can leave its lock file behind. DotForge then stops with
  `pacman is busy` (exit code 1) and tells you how to remove the lock.
- **AUR packages** are built by your AUR helper, which may ask its own
  questions (for example to review a PKGBUILD) unless it is configured
  not to.
- **One profile per machine** is the expected use. Applying a second
  profile works, but DotForge doesn't merge them: each profile tracks only
  what it applied itself.
- **Config links are symlinks.** A few programs replace their config file
  instead of editing it, which turns the link back into a normal file.
  DotForge shows this as "a file is there now" on the next apply and backs
  it up before linking again.
- **Setup actions** are limited to `enable_service`, `add_user_to_group`
  and `set_default_shell`. Anything else needs a catalog entry, or a
  custom script as a last resort.
- **Logging out** is needed after joining a group or changing your shell.
  DotForge reminds you.
