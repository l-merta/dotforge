# Command line conventions

These rules apply to **every** DotForge command, now and in the future.

## `--json`

Add `--json` anywhere in the command to get JSON instead of text:

```sh
dotforge --json catalog show kitty
dotforge catalog show kitty --json
```

The JSON always has the same shape, and is always printed to stdout.

Success:

```json
{ "ok": true, "exit_code": 0, "data": { ... } }
```

The command ran, but found problems (for example `validate`):

```json
{ "ok": false, "exit_code": 4, "data": { "errors": 2, "issues": [ ... ] } }
```

The command could not run:

```json
{
  "ok": false,
  "exit_code": 3,
  "error": { "kind": "not_found", "message": "...", "hint": "..." }
}
```

`error.kind` is one of `not_found`, `already_exists`, `io`, `parse`,
`invalid`, `editor`, `no_config_dir`, `not_implemented`. Parse errors also include `file`,
`field`, `line`, `column` and `reason`.

In text mode, results go to stdout and errors to stderr.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | General failure (for example a file could not be read). |
| 2 | Wrong command line (unknown command, missing argument). |
| 3 | The profile or catalog entry was not found. |
| 4 | A profile or catalog file is invalid, or validation found errors. |
| 5 | The command is not implemented yet. |
| 6 | Something with that name already exists (for example `profile create` for an existing profile). |

These numbers will not change. New codes may be added later.

## Files and folders

| What | Where |
|---|---|
| Your profiles | `~/.config/dotforge/profiles/<name>/profile.yml` |
| Your catalog entries | `~/.config/dotforge/catalog/` |
| DotForge settings | `~/.config/dotforge/config.yml` |

If `$XDG_CONFIG_HOME` is set, `~/.config` is replaced by it. Setting
`$DOTFORGE_CONFIG_DIR` points DotForge at a completely different folder
(the tests use this). `$DOTFORGE_SYSTEM_ROOT` does the same for the system
files DotForge reads, such as `/etc/os-release` (see
[system-and-status.md](system-and-status.md)).

## Settings

DotForge's own settings live in `~/.config/dotforge/config.yml`. The
file is optional: every setting has a default. Unknown settings are
errors, so typos are caught.

```yaml
editor: nano
```

| Setting | Default | Meaning |
|---|---|---|
| `editor` | `nano` | The program `dotforge profile edit` and `dotforge config edit` open files with. Arguments are allowed, split by spaces: `code --wait`, `kate -b`. The file path is added at the end. |

```sh
dotforge config show    # the settings in use, and whether each is a default
dotforge config edit    # open config.yml (created from a template if missing)
```

Graphical editors usually return immediately, before you have finished
editing. Give them their "wait" option (`code --wait`, `kate -b`,
`subl -w`) so DotForge can check the file after you close it.
