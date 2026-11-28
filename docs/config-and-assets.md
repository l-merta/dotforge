# Config files and assets

A profile folder can hold your own config files and assets (wallpapers,
fonts, icons, cursors). `dotforge profile apply` puts them in place with
**symlinks**: the files in the profile folder are the originals, and the
places apps look for them point back at the profile. Edit
`~/.config/kitty/kitty.conf` and you are editing the file in your profile.
The profile folder is the one place that holds your whole setup.

## Where things go

```
profiles/desktop/
├── profile.yml
├── config/
│   ├── hyprland/          ->  ~/.config/hypr
│   ├── kitty/             ->  ~/.config/kitty
│   └── home/              ->  your home folder, file by file
│       ├── .zshrc         ->  ~/.zshrc
│       └── .config/git/config  ->  ~/.config/git/config
└── assets/
    ├── wallpapers/        ->  ~/.local/share/dotforge/wallpapers
    ├── fonts/             ->  ~/.local/share/fonts/dotforge
    ├── icons/Papirus/     ->  ~/.local/share/icons/Papirus
    └── cursors/Bibata/    ->  ~/.local/share/icons/Bibata
```

Empty folders are skipped. `profile create` makes empty `assets/`
folders for you to fill, and nothing is linked until you put something in
them.

### `config/<catalog id>/`

A folder named after a catalog entry holds that entry's config. The
catalog knows where it goes (`dotforge catalog show kitty` shows
`Configuration: ~/.config/kitty`), and the **whole folder** is linked.

- The entry must be in the profile (or be needed by something in it).
  Otherwise its config is not linked, and `profile validate` warns you.
- A folder name that isn't a catalog ID is an error, with a "did you
  mean" tip.
- Entries whose config location the catalog doesn't know (for example
  `git`) can't use this. Use `config/home/` for them.

### `config/home/`

Anything else. It mirrors your home folder, and each **file** is linked on
its own, so `config/home/.config/git/config` links only that file, not the
whole `~/.config`.

### `assets/`

| Folder | Linked to | Notes |
|---|---|---|
| `assets/wallpapers/` | `~/.local/share/dotforge/wallpapers` | A fixed path your configs can point at, for example in `hyprpaper.conf`. |
| `assets/fonts/` | `~/.local/share/fonts/dotforge` | Apps usually see new fonts right away; if not, run `fc-cache -f`. |
| `assets/icons/<theme>/` | `~/.local/share/icons/<theme>` | Each theme is its own folder. |
| `assets/cursors/<theme>/` | `~/.local/share/icons/<theme>` | Cursor themes live next to icon themes. |

## When something is already there

DotForge never silently replaces your files. When a file, folder or other
link is already where a link should go:

1. The plan says so: `a folder is there now and will be moved to ...`.
2. After you confirm, it is **moved** (not deleted) to
   `~/.local/state/dotforge/backups/<date>-<time>/`, keeping its path
   relative to your home. For example `~/.config/kitty` goes to
   `backups/20261125-201001/.config/kitty`.
3. Then the link is created.

To undo, delete the link and move the backup back:

```sh
rm ~/.config/kitty
mv ~/.local/state/dotforge/backups/20261125-201001/.config/kitty ~/.config/kitty
```

## Running it again

- A link that already points at the profile is left alone (`already
  linked`), so applying twice does nothing the second time.
- If you remove a config folder or file from the profile, the next apply
  removes the link it made. Only the link is removed; your file stays in
  the profile folder. DotForge only removes links that still point at the
  profile: if you replaced one with your own file, it is left alone.

## Conflicts

Two things in the profile can't own the same place. For example,
`config/hyprland/` (all of `~/.config/hypr`) and
`config/home/.config/hypr/extra.conf` (a file inside it) conflict. This is
an error in `profile validate`, and `apply` refuses to run until one of
them is removed. Put the file in `config/hyprland/` instead.

## Themes

Themes and larger config bundles are **not** catalog entries for now. A
"theme" is simply what you put in a profile's `config/` and `assets/`.
Sharing themes may become a catalog feature later.
