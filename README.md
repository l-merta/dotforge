# DotForge

A Linux desktop setup and rice manager. You describe the computer you want
in a YAML **profile** (desktop, compositor, shell, apps, drivers, ...),
DotForge looks each piece up in its **catalog**, and, eventually, makes
your machine match the profile.

> **Work in progress.** DotForge is an early prototype. It can read and
> check profiles and catalog entries, but it does not install or change
> anything yet. Arch Linux is the first supported system.

## Build and install from source

You need [Rust](https://rustup.rs/).

```sh
git clone https://github.com/l-merta/dotforge
cd dotforge
cargo install --path .
```

This puts the `dotforge` program in `~/.cargo/bin`.

## Try it

```sh
dotforge catalog list                 # everything DotForge knows about
dotforge catalog show hyprland        # details of one entry
dotforge catalog validate             # check the catalog for mistakes
dotforge profile validate ./profiles/test   # check a profile
dotforge --json catalog list          # any command, as JSON
```

## Documentation

- [Profile format](docs/profile-format.md)
- [Catalog format](docs/catalog-format.md)
- [Command line conventions: `--json`, exit codes, file locations](docs/cli.md)
