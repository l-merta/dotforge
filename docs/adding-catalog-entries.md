# Adding a catalog entry

This guide shows how to add a new program to the catalog, step by step.
The full field reference is in [catalog-format.md](catalog-format.md).

## 1. Decide where it goes

- **Just for you:** put the file in `~/.config/dotforge/catalog/<folder>/`.
  It works straight away, no rebuild needed.
- **For everyone (built-in):** put it in this repository's
  `catalog/<folder>/` and open a pull request.

`<folder>` is the plural of the category: `desktops`, `compositors`,
`shells`, `components` or `applications`.

## 2. Find the package name

On Arch:

```sh
pacman -Ss <name>     # official repositories → goes under `packages`
paru -Ss <name>       # also searches the AUR → goes under `aur`
```

Prefer official packages. For AUR packages, prefer the maintained `-bin`
package when the normal one has to be compiled for a long time.

## 3. Write the file

Example `catalog/applications/obsidian.yml`:

```yaml
id: obsidian
name: Obsidian
description: Note-taking app based on Markdown files
category: application

platforms: [linux]
protocols: [wayland, x11]

installation:
  arch:
    packages: [obsidian]

configuration:
  directory: ~/.config/obsidian
```

Checklist:

- [ ] `id` matches the file name and uses lowercase letters, digits and dashes.
- [ ] `category` matches the folder.
- [ ] Desktops, compositors and shells list their `protocols`.
- [ ] Components list `capabilities` (what job they do).
- [ ] `dependencies` are catalog IDs that exist (add them first if not).
- [ ] Extra steps (services, groups, login shell) are `setup` actions.
- [ ] Descriptions with `: ` in them are in `"quotes"`.

## 4. Check it

```sh
dotforge catalog validate
dotforge catalog show obsidian
dotforge catalog deps obsidian
```

For a built-in entry, also run the tests. They fail if any built-in entry
has an error or warning:

```sh
cargo test
```

## 5. Overriding a built-in entry

To use a different package for something that is already built in, create
a file with the **same ID** in your own catalog folder. Yours replaces the
built-in one, and `catalog list` marks it `[user]`. For example,
`~/.config/dotforge/catalog/applications/vscode.yml` with
`packages: [code]` uses the open-source build instead of Microsoft's.
