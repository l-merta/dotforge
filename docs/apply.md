# Applying a profile

`dotforge profile apply <profile>` makes this machine match a profile. It
always works in four steps:

1. **Plan.** DotForge compares the profile with the machine (the same
   comparison as `profile status`) and works out exactly which commands
   it would run.
2. **Confirm.** It shows you the plan and asks `Apply these changes? [y/N]`.
   Anything except `y` or `yes` cancels, and nothing is changed.
3. **Execute.** It runs the commands one by one.
4. **Report.** It shows what worked, what failed and what was skipped.

```sh
dotforge profile apply desktop --dry-run   # only show the plan
dotforge profile apply desktop             # show the plan, ask, then apply
dotforge profile apply desktop --yes       # don't ask (for scripts)
```

Only the Arch family is supported for now. On another system `apply`
stops with exit code 4 before doing anything.

## What it runs

| Step | Command |
|---|---|
| Official packages | `sudo pacman -S --needed --noconfirm <packages>` |
| AUR packages | `paru -S --needed --noconfirm <packages>` (or `yay`), as you, not root |
| `enable_service` | `sudo systemctl enable --now <service>` (or `systemctl --user enable --now` for user services) |
| `add_user_to_group` | `sudo usermod -aG <group> <you>` |
| `set_default_shell` | `sudo usermod --shell <shell> <you>` |
| Removing a dropped item | `sudo pacman -Rs --noconfirm <packages>` |
| Config files and assets | symlinks from your home folder into the profile folder, made by DotForge itself (see [config-and-assets.md](config-and-assets.md)) |

`--noconfirm` is safe here because DotForge has already asked you. All the
packages go into **one** pacman command, so pacman resolves them together.

**Run DotForge as your normal user, not with `sudo`.** It asks for your
password (through `sudo`) only for the steps that need it. AUR helpers
refuse to run as root, so `sudo dotforge ...` is refused with exit code 4.

## Running it again is safe

Every step is checked before it runs:

- packages that are already installed are left alone;
- a service that is already enabled, a group you are already in, or a
  shell that is already your login shell is shown as `already done`.

So running `apply` twice in a row does nothing the second time:
`This machine already matches the profile. Nothing to do.`

If a step fails, the steps that depend on it are **skipped** (for example,
the "set Zsh as your shell" step when installing Zsh failed). Fix the
problem and run `apply` again: what already worked is not repeated.

## Items you removed from the profile

DotForge remembers what it applied for each profile in
`~/.local/state/dotforge/state.json`. When you take something out of a
profile and apply again, DotForge notices it and asks:

```
'htop' is no longer in the profile but still installed (htop). What should DotForge do? [r]emove / [i]gnore / [k]eep for now (default)
```

- **remove** uninstalls its packages (only the ones no other item in the
  profile still needs) and forgets it;
- **ignore** leaves it installed and forgets it, so you're not asked again;
- **keep** (the default, just press Enter) leaves it installed and asks
  again next time.

`--yes` always chooses **keep**: DotForge never removes anything without
you saying so.

## Custom scripts

Sometimes you need a step DotForge has no catalog entry or setup action
for yet. For that, a profile can list its own scripts, kept in the
profile folder:

```
profiles/desktop/
├── profile.yml
└── scripts/
    └── tweaks.sh
```

```yaml
scripts: [scripts/tweaks.sh]
```

Scripts are an **escape hatch**, and DotForge treats them as unsafe,
because it can't know what they do, check them or undo them:

- They are always listed in the plan under **Your own scripts**.
- `--dry-run` only shows them. It never runs them.
- Before each script runs, DotForge asks
  `Run your script scripts/tweaks.sh (new, 12 line(s))? ... [y/N]`.
  This question is asked **even with `--yes`**; with no answer (for
  example in a script or CI) the answer is no and the script is skipped.
- A script runs with `bash`, as you (not root), from the profile folder,
  after everything else. If an earlier step failed, scripts are skipped.
- A script that ran successfully runs again only when you **change** it
  (DotForge remembers a fingerprint of its contents). A script that failed
  or that you said no to is asked about again next time.
- Scripts must live inside the profile folder, so a profile stays
  self-contained. `profile validate` reports paths outside it and missing
  files.

Scripts don't replace setup actions. If you find yourself writing the same
kind of script often, it should become a catalog entry or a setup action.

## Logs

Every run that changes something writes a log file to
`~/.local/state/dotforge/logs/apply-<date>-<time>.log` with each command
and its full output. The report prints the log's path. The output of the
commands is also shown live while they run.

## For scripts: `--json`

```sh
dotforge --json profile apply desktop --yes
```

`data.result` is one of:

| `result` | Meaning |
|---|---|
| `dry_run` | `--dry-run` was used; `data.plan` is the plan. |
| `nothing_to_do` | The machine already matched. |
| `applied` | Changes were made; `data.report` lists every step with `status` `ok`, `failed` or `skipped`. |

`data.plan.scripts` lists each script with `status` `new`, `changed` or
`unchanged`.
| `cancelled` | You answered no (exit code 7). |

Exit codes: 0 everything worked, 1 at least one step failed, 4 the profile
is invalid or the system isn't supported, 7 cancelled.

## Where things are stored

| What | Where |
|---|---|
| What DotForge applied | `~/.local/state/dotforge/state.json` |
| Logs | `~/.local/state/dotforge/logs/` |
| Files moved out of the way of a link | `~/.local/state/dotforge/backups/` |

`$XDG_STATE_HOME` replaces `~/.local/state` when it is set, and
`$DOTFORGE_STATE_DIR` points at a different folder entirely (the tests use
it).
