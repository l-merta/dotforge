# DotForge

A Linux desktop setup and rice manager. You describe the computer you want
in a YAML **profile** (desktop, compositor, shell, apps, drivers, ...),
DotForge looks each piece up in its **catalog**, and, eventually, makes
your machine match the profile.

> **Work in progress.** DotForge is an early prototype. It can create, list and
> check profiles, browse a catalog of about 40 desktop components and apps,
> compare a profile with your machine, install what is missing, and link
> your config files and assets from the profile folder. Arch Linux is the first supported
> system. Try `apply` in a virtual machine first.

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
dotforge profile status desktop       # compare it with this machine (read-only)
dotforge profile apply desktop        # install what's missing (asks first)
dotforge system show                  # what DotForge detects about this machine
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
- [System detection and profile status](docs/system-and-status.md)
- [Applying a profile](docs/apply.md)
- [Config files and assets](docs/config-and-assets.md)
- [Hand tests: try each feature yourself](docs/manual-tests.md)
