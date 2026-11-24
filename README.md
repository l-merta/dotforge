# DotForge

A Linux desktop setup and rice manager. You describe the computer you want
in a YAML **profile** (desktop, compositor, shell, apps, drivers, ...),
DotForge looks each piece up in its **catalog**, and, eventually, makes
your machine match the profile.

> **Work in progress.** DotForge is an early prototype. It can create, list and
> check profiles and browse a catalog of about 40 desktop components and apps, but it does not install or change
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
dotforge profile create desktop       # make your first profile
dotforge profile list                 # see your profiles
dotforge profile show desktop         # what a profile contains
dotforge profile edit desktop         # open it in your editor (nano by default)
dotforge profile validate desktop     # check it for mistakes
dotforge catalog list                 # everything DotForge knows about
dotforge catalog show hyprland        # details of one entry
dotforge catalog deps hyprland        # what it needs, in install order
dotforge --json profile list          # any command, as JSON
```

Your profiles live in `~/.config/dotforge/profiles/`. DotForge's own
settings (such as which editor to use) are in `~/.config/dotforge/config.yml`;
see them with `dotforge config show` and change them with `dotforge config edit`.

## Documentation

- [Profile format](docs/profile-format.md)
- [Catalog format](docs/catalog-format.md)
- [Adding a catalog entry](docs/adding-catalog-entries.md)
- [Command line conventions: `--json`, exit codes, file locations](docs/cli.md)
- [Hand tests: try each feature yourself](docs/manual-tests.md)
