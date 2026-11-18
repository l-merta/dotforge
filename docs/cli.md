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
`invalid`, `no_config_dir`, `not_implemented`. Parse errors also include `file`,
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

If `$XDG_CONFIG_HOME` is set, `~/.config` is replaced by it. Setting
`$DOTFORGE_CONFIG_DIR` points DotForge at a completely different folder
(the tests use this).
