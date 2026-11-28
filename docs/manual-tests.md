# Hand tests

Simple checks you can run yourself to see each feature working. Every
step shows the command to type and what you should see.

`echo $?` prints the **exit code** of the command you just ran (the
number a program gives back when it finishes; 0 means success). The
meaning of each code is in [cli.md](cli.md#exit-codes).

## Before you start

```sh
cd ~/path/to/dotforge          # the folder where you cloned the repo
git pull                       # get the latest version
cargo install --path .         # build and install the `dotforge` program
export DOTFORGE_CONFIG_DIR=/tmp/dotforge-test
```

The last line is important. It makes DotForge use a **practice folder**
in `/tmp` instead of your real `~/.config/dotforge/`, so nothing you do
here touches your real setup. It only lasts until you close the terminal,
so run it again in each new terminal.

When you are done, clean up with:

```sh
rm -rf /tmp/dotforge-test /tmp/broken /tmp/fake-ubuntu
unset DOTFORGE_CONFIG_DIR
```

---

## Milestone 1: catalog, validation, `--json`, exit codes

### 1.1 Browse the catalog

```sh
dotforge catalog list
```

You see `Catalog entries: 39`, then the five categories: Desktop
environments (GNOME, KDE Plasma), Compositors (Hyprland, Niri, Sway),
Shells (Noctalia), Components and Applications.

### 1.2 Filter by category

```sh
dotforge catalog list compositor
```

Only Hyprland, Niri and Sway are shown.

### 1.3 Look at one entry

```sh
dotforge catalog show hyprland
```

You see Hyprland's ID, category, description, `Source: built-in
(compositors/hyprland.yml)`, capabilities, dependencies, packages, setup
steps and config folder.

### 1.4 A typo gets a suggestion

```sh
dotforge catalog show hyprlnd
echo $?
```

You see `error: catalog entry 'hyprlnd' was not found` and `hint: did
you mean 'hyprland'?`. The exit code is **3** (not found).

### 1.5 Check the whole catalog

```sh
dotforge catalog validate
echo $?
```

You see `Checked 39 catalog entries: 0 error(s), 0 warning(s)` and
`The catalog is valid.` The exit code is **0**.

### 1.6 JSON output

```sh
dotforge --json catalog show kitty
dotforge catalog show kitty --json
```

Both print the same JSON. It starts with `"data": {` and ends with
`"exit_code": 0` and `"ok": true`.

### 1.7 Check the example profile in the repo

```sh
dotforge profile validate ./profiles/test
echo $?
```

You see `0 error(s), 0 warning(s)` and `The profile is valid.` The exit
code is **0**. (Before Milestone 3 this showed 17 errors, because apps
like Steam weren't in the catalog yet.)

### 1.8 Errors point to the exact line and field

```sh
mkdir -p /tmp/broken
printf 'format_version: 1\nname: Broken\ndevice:\n  type: desktop\n  os:\n    family: windows\n    distribution: arch\n' > /tmp/broken/profile.yml
dotforge profile validate /tmp/broken
```

You see:

```
error: /tmp/broken/profile.yml:6:13: problem in field 'device.os.family': unknown variant `windows`, expected `arch`
```

That means: line 6, column 13, field `device.os.family`.

### 1.9 It works from any folder

```sh
cd /
dotforge catalog list compositor
cd -
```

You get the same list as in 1.2. The catalog is built into the program,
so it doesn't matter which folder you are in.

### 1.10 Your own catalog entry replaces a built-in one

```sh
mkdir -p $DOTFORGE_CONFIG_DIR/catalog/applications
printf 'id: kitty\nname: My Kitty\ndescription: My own version of Kitty\ncategory: application\ninstallation:\n  arch:\n    packages: [kitty]\n' > $DOTFORGE_CONFIG_DIR/catalog/applications/kitty.yml
dotforge catalog list application
dotforge catalog show kitty
```

The list shows `kitty - My Kitty  [user]`, and `show` prints `My Kitty`
with your description. Remove it again with:

```sh
rm -r $DOTFORGE_CONFIG_DIR/catalog
```

### 1.11 A wrong command

```sh
dotforge catalog fly
echo $?
```

You see `error: unrecognized subcommand 'fly'`. The exit code is **2**
(wrong command line).

### 1.12 DotForge's own settings

```sh
dotforge config show
```

You see `Settings file: /tmp/dotforge-test/config.yml (not created yet,
using defaults)` and `editor: nano  (default)`.

```sh
dotforge config edit
```

Nano opens a new settings file with `editor: nano` in it. Change it to an
editor you like (for example `editor: micro`, or `editor: code --wait`
for VS Code), save (Ctrl+O, Enter) and quit (Ctrl+X). You see `Saved
.../config.yml (new file). The settings are valid.` Run `dotforge config
show` again: the editor now shows your choice, without `(default)`.

Now make a typo on purpose:

```sh
echo "editr: vim" > $DOTFORGE_CONFIG_DIR/config.yml
dotforge config show
echo $?
```

You see an error about `editr` (an unknown field) and exit code **4**.
Fix it with `echo "editor: nano" > $DOTFORGE_CONFIG_DIR/config.yml`.

---

## Milestone 2: profile management

### 2.1 No profiles yet

```sh
dotforge profile list
```

You see `No profiles yet in /tmp/dotforge-test/profiles.` and a tip to
use `dotforge profile create <name>`.

### 2.2 Create a profile

```sh
dotforge profile create desktop --description "Main PC"
ls -R $DOTFORGE_CONFIG_DIR/profiles/desktop
```

You see `Created profile 'desktop'` and three next steps. The `ls`
shows `profile.yml`, a `config` folder, and an `assets` folder with
`cursors`, `fonts`, `icons` and `wallpapers` in it.

### 2.3 Creating it twice is refused

```sh
dotforge profile create desktop
echo $?
```

You see `error: profile 'desktop' already exists` and where it is. The
exit code is **6** (already exists). Your profile is not changed.

### 2.4 Bad names get a suggestion

```sh
dotforge profile create "My Laptop"
echo $?
```

You see that `'My Laptop' is not a valid profile name` with the hint
`try 'my-laptop'`. The exit code is **4**.

### 2.5 Create a second profile with options

```sh
dotforge profile create laptop --device laptop --display-name "Work laptop"
```

### 2.6 List your profiles

```sh
dotforge profile list
```

You see both profiles:

```
  desktop  desktop - Main PC
  laptop   Work laptop
```

### 2.7 Show a profile

```sh
dotforge profile show desktop
```

You see `Main PC`, the file path, and the sections Device, System,
Applications and Rice. Fields you haven't filled in show `-`.

### 2.8 Edit the profile, then check it

Open the file in an editor:

```sh
nano $DOTFORGE_CONFIG_DIR/profiles/desktop/profile.yml
```

At the bottom, remove the `# ` at the start of each line of the `rice:`
block, save (Ctrl+O, Enter) and quit (Ctrl+X). (Shortcut without an
editor: `sed -i '/^# rice:/,$ s/^# //' $DOTFORGE_CONFIG_DIR/profiles/desktop/profile.yml`)

```sh
dotforge profile validate desktop
dotforge profile show desktop
```

You see `The profile is valid.`, and `show` now lists `hyprland` as the
compositor and `noctalia` as the shell.

Now add a typo:

```sh
sed -i 's/compositor: hyprland/compositor: hyprlnd/' $DOTFORGE_CONFIG_DIR/profiles/desktop/profile.yml
dotforge profile validate desktop
echo $?
```

You see `error: rice.compositor: 'hyprlnd' is not in the catalog (did
you mean 'hyprland'?)` and the exit code is **4**. Fix it back with
`sed -i 's/hyprlnd/hyprland/' $DOTFORGE_CONFIG_DIR/profiles/desktop/profile.yml`.

### 2.9 Broken profiles don't break the list

```sh
mkdir -p $DOTFORGE_CONFIG_DIR/profiles/empty
mkdir -p $DOTFORGE_CONFIG_DIR/profiles/Broken
echo "name: [oops" > $DOTFORGE_CONFIG_DIR/profiles/Broken/profile.yml
dotforge profile list
echo $?
```

All four profiles are listed. `Broken` and `empty` are marked
`[problem]` with the reason (bad YAML, and `the folder has no
profile.yml`). Below the list there is a warning that `Broken` breaks the
naming rules, suggesting `broken`. The exit code is still **0**.

### 2.10 A folder without `profile.yml`

```sh
dotforge profile show empty
echo $?
```

You see `error: profile 'empty' was not found` with the hint `the folder
exists but has no 'profile.yml' in it`. The exit code is **3**.

### 2.11 Profile commands as JSON

```sh
dotforge --json profile list
```

You get JSON with one object per profile, each with `name`, `status`
(`ok`, `missing_file` or `unreadable`), `path` and so on.

### 2.12 Not built yet

```sh
dotforge profile apply desktop
echo $?
```

You see `'profile apply' is not implemented yet (planned for Milestone 5)`.
The exit code is **5**.

### 2.13 Edit a profile in your editor

```sh
dotforge profile edit desktop
```

Your editor (nano unless you changed it in 1.12) opens the desktop
profile. Change the description, save and quit. You see `Saved ...` and
then the same check as `profile validate`, ending with `The profile is
valid.`

Now try an editor that doesn't exist:

```sh
echo "editor: no-such-editor" > $DOTFORGE_CONFIG_DIR/config.yml
dotforge profile edit desktop
echo $?
```

You see `could not open the editor 'no-such-editor': the program
'no-such-editor' was not found`, with a hint pointing at `config.yml`.
The exit code is **1**. Put it back with
`echo "editor: nano" > $DOTFORGE_CONFIG_DIR/config.yml`.

---

## Milestone 3: a bigger catalog, AUR, setup actions, dependencies

### 3.1 The catalog has grown

```sh
dotforge catalog list
dotforge catalog validate
```

The list shows 39 entries, and validate shows `0 error(s), 0 warning(s)`.

### 3.2 AUR packages are marked

```sh
dotforge catalog show heroic
```

Under `Installation:` you see `heroic-games-launcher-bin  (AUR)`. That
means it will come from the AUR through paru or yay, not from pacman.
`dotforge catalog show steam` shows a normal package (no `(AUR)`).

### 3.3 Setup steps

```sh
dotforge catalog show docker
```

Under `Setup steps:` you see `enable the system service docker.service`
and `add you to the 'docker' group`. These are the extra things
installing Docker needs. Try `dotforge catalog show zsh` too: it says
`make /usr/bin/zsh your login shell`.

### 3.4 Dependencies in install order

```sh
dotforge catalog deps hyprland
```

You see 3 numbered steps: `xdg-desktop-portal`, then
`xdg-desktop-portal-hyprland`, then `hyprland`. Dependencies always come
before the thing that needs them. Try `dotforge catalog deps kde-plasma`:
it installs the SDDM login screen first and enables its service.

### 3.5 A typo in `deps`

```sh
dotforge catalog deps hyprlnd
echo $?
```

You see `did you mean 'hyprland'?` and the exit code is **3**.

### 3.6 Circular dependencies are caught

Make two entries that need each other:

```sh
mkdir -p $DOTFORGE_CONFIG_DIR/catalog/applications
printf 'id: chicken\nname: Chicken\ndescription: Came first?\ncategory: application\nplatforms: [linux]\ninstallation:\n  arch:\n    packages: [chicken]\ndependencies: [egg]\n' > $DOTFORGE_CONFIG_DIR/catalog/applications/chicken.yml
printf 'id: egg\nname: Egg\ndescription: Or did I?\ncategory: application\nplatforms: [linux]\ninstallation:\n  arch:\n    packages: [egg]\ndependencies: [chicken]\n' > $DOTFORGE_CONFIG_DIR/catalog/applications/egg.yml
dotforge catalog validate
echo $?
dotforge catalog deps chicken
```

Validate shows `error: chicken: circular dependency: chicken → egg →
chicken` and exit code **4**. `deps` shows the same circle. Clean up:

```sh
rm -r $DOTFORGE_CONFIG_DIR/catalog
```

### 3.7 Shell commands are not allowed in the catalog

```sh
mkdir -p $DOTFORGE_CONFIG_DIR/catalog/applications
printf 'id: sneaky\nname: Sneaky\ndescription: Tries to run a command\ncategory: application\nplatforms: [linux]\ninstallation:\n  arch:\n    packages: [sneaky]\nsetup:\n  - action: run_command\n    command: echo hi\n' > $DOTFORGE_CONFIG_DIR/catalog/applications/sneaky.yml
dotforge catalog list
echo $?
```

You see an error that points at line 10, field `setup[0].action`:
`unknown variant `run_command`, expected one of `enable_service`,
`add_user_to_group`, `set_default_shell``. The exit code is **4**. Only
the three typed actions are allowed. Clean up with
`rm -r $DOTFORGE_CONFIG_DIR/catalog`.

### 3.8 The wrong tool for a job gets a warning

```sh
dotforge profile create rice-test > /dev/null
F=$DOTFORGE_CONFIG_DIR/profiles/rice-test/profile.yml
sed -i '/^# rice:/,$ s/^# //' $F
sed -i 's/notifications: noctalia/launcher: dunst/' $F
dotforge profile validate rice-test
echo $?
```

You see `warning: rice.components.launcher: 'dunst' does not list the
'launcher' capability`. Dunst shows notifications, it isn't an app
launcher. It's only a warning, so the profile still counts as valid and
the exit code is **0**.

### 3.9 Dependencies as JSON

```sh
dotforge --json catalog deps docker
```

You get JSON with an `order` list. Each step has `id`, `packages`, `aur`
and `setup` (with `"action": "enable_service"` and so on).

---

## Milestone 4: detecting your system and comparing it with a profile

These commands only **read** your system. They never install or change
anything, so it is safe to run them on your real machine.

### 4.1 What DotForge sees

```sh
dotforge system show
```

You see your distribution (for example `Arch Linux` or `EndeavourOS`),
`DotForge family: arch`, `package manager: pacman`, the number of
installed packages, your AUR helper (`yay` or `paru` if you have one), the
enabled repositories (`core, extra`, plus `multilib` if you turned it on)
and `systemd: yes`. In a desktop session, `session` is `wayland` or `x11`.

### 4.2 The package count is right

```sh
pacman -Q | wc -l
```

This prints the same number as `installed packages` in 4.1.

### 4.3 Compare the example profile with your machine

```sh
dotforge profile status ./profiles/test
echo $?
```

You see one line per program the profile needs, dependencies included,
in install order:

- `[ok  ]` means it is already installed;
- `[need]` means something is missing, and the line says which package
  (`(AUR)` if it comes from the AUR).

Then there is a summary (`... installed, ... missing`), the packages
DotForge **would** install (split into pacman and your AUR helper), the
setup steps, and `Nothing was changed.` The exit code is **0**.

If `multilib` isn't in your repositories from 4.1, there is also a
warning that Steam needs the `multilib` repository.

### 4.4 A profile you already match

```sh
mkdir -p $DOTFORGE_CONFIG_DIR/profiles/mine
printf 'format_version: 1\nname: Mine\ndevice:\n  type: desktop\n  os:\n    family: arch\n    distribution: arch\napplications:\n  development: [git]\n' > $DOTFORGE_CONFIG_DIR/profiles/mine/profile.yml
dotforge profile status mine
```

If Git is installed (`pacman -Q git` shows it), you see `[ok  ] git` and
`This machine already matches the profile.` If it isn't, you see
`[need] git` and `with pacman: git`.

### 4.5 Packages that "provide" another one count too

```sh
pacman -Qi bash | grep Provides
```

You see `Provides        : sh`, which means Bash also counts as "sh".
Many packages provide other names like this. For example, `hyprland-git`
provides `hyprland`. DotForge counts a provided name as installed, so if you use a
`-git` version of something in a profile, `profile status` still shows it
as `[ok  ]`.

### 4.6 A system DotForge doesn't support yet

Make a fake Ubuntu system and point DotForge at it:

```sh
mkdir -p /tmp/fake-ubuntu/etc
printf 'NAME="Ubuntu"\nID=ubuntu\nID_LIKE=debian\nVERSION_ID="24.04"\n' > /tmp/fake-ubuntu/etc/os-release
DOTFORGE_SYSTEM_ROOT=/tmp/fake-ubuntu dotforge system show
DOTFORGE_SYSTEM_ROOT=/tmp/fake-ubuntu dotforge profile status ./profiles/test
```

`system show` says `Ubuntu 24.04`, `DotForge family: not supported yet`,
and a note that only the Arch family is supported. `profile status` marks
every line `[??  ]` (can't check here) and explains why.
`DOTFORGE_SYSTEM_ROOT` only applies to the command it is written in front
of.

### 4.7 A profile with errors is refused

```sh
sed -i 's/development: \[git\]/development: [gti]/' $DOTFORGE_CONFIG_DIR/profiles/mine/profile.yml
dotforge profile status mine
echo $?
```

You see that the profile has 1 error and can't be compared yet, with a
hint to run `dotforge profile validate mine`. The exit code is **4**.

### 4.8 Status as JSON

```sh
dotforge --json profile status ./profiles/test | grep -E '"satisfied"|"(installed|missing|unsupported)": [0-9]'
```

You see `"satisfied": false` (unless you have everything installed!) and
the `installed` and `missing` counts. Scripts read `data.to_install` for
the exact package lists.

## Milestone 5: applying a profile (installing for real)

**These tests install and remove packages.** Run them in the `arch-test`
virtual machine, not on your real computer. Before you start:

1. In VMware, revert `arch-test` to the `clean-baseline` snapshot.
2. Start it and log in as `arch`.
3. Get this version of DotForge:

```sh
cd ~/dotforge
git fetch && git checkout milestone-5-apply-engine && git pull
cargo build --release
```

`dotforge` already points at `~/dotforge/target/release/dotforge`, so the
new build is used right away. Don't set `DOTFORGE_CONFIG_DIR` here: in the
VM we use the real `~/.config/dotforge`.

Make a small profile to play with:

```sh
mkdir -p ~/.config/dotforge/profiles/vm
cat > ~/.config/dotforge/profiles/vm/profile.yml <<'YAML'
format_version: 1
name: VM test
device:
  type: desktop
  os:
    family: arch
    distribution: arch
applications:
  utilities: [htop, fastfetch, zsh]
YAML
```

### 5.1 Dry run: see the plan, change nothing

```sh
dotforge profile apply vm --dry-run
pacman -Q htop
```

You see `Install (3): htop fastfetch zsh` with the exact pacman command,
a setup step `make /usr/bin/zsh your login shell`, and
`Dry run: nothing was changed.` Then `pacman -Q htop` says
`package 'htop' was not found`: nothing was installed.

### 5.2 Saying no cancels

```sh
dotforge profile apply vm
```

Type `n` and press Enter. You see `Cancelled. Nothing was changed.`
`echo $?` prints **7**.

### 5.3 Apply for real

```sh
dotforge profile apply vm
```

Type `y`. You see pacman's own output scroll by, then a result like:

```
  [ok     ] install 3 package(s): htop, fastfetch, zsh
  [ok     ] make /usr/bin/zsh your login shell (zsh)
```

and `The profile was applied.` Check it really happened:

```sh
htop --version
getent passwd arch      # ends with /usr/bin/zsh
```

### 5.4 Running it again does nothing

```sh
dotforge profile apply vm
```

You see `This machine already matches the profile. Nothing to do.` and
it doesn't ask anything.

### 5.5 The log

```sh
ls ~/.local/state/dotforge/logs/
less ~/.local/state/dotforge/logs/apply-*.log
```

There is one log per apply run that changed something, with every command
and its full output.

### 5.6 Removing something from the profile

Take `htop` out of the profile:

```sh
sed -i 's/htop, //' ~/.config/dotforge/profiles/vm/profile.yml
dotforge profile apply vm
```

The plan says `'htop' was removed from the profile but is still
installed`, then DotForge asks what to do. Just press **Enter** (keep).
htop stays installed (`pacman -Q htop` still finds it).

Run it again and this time type **r**:

```sh
dotforge profile apply vm
pacman -Q htop
```

You see `[ok     ] remove htop`, and `pacman -Q htop` now says it is not
found. Run apply once more: `Nothing to do`, it doesn't ask again.

### 5.7 `--yes` never removes anything

```sh
sed -i 's/utilities: \[/utilities: [htop, /' ~/.config/dotforge/profiles/vm/profile.yml
dotforge profile apply vm --yes          # installs htop again, no question
sed -i 's/htop, //' ~/.config/dotforge/profiles/vm/profile.yml
dotforge profile apply vm --yes
pacman -Q htop
```

The second apply doesn't ask anything and doesn't remove htop: `--yes`
always picks "keep" for removed items.

### 5.8 An AUR package

paru is already in the VM. Add an AUR-only app (Visual Studio Code is
`visual-studio-code-bin` in the AUR):

```sh
sed -i 's/utilities: \[/utilities: [htop, /; s/^applications:/applications:\n  development: [vscode]/' ~/.config/dotforge/profiles/vm/profile.yml
cat ~/.config/dotforge/profiles/vm/profile.yml
dotforge profile apply vm
```

The plan shows a separate `Install from the AUR` part that runs
`paru -S --needed --noconfirm visual-studio-code-bin` **without** sudo.
Type `y`. paru builds and installs it (this takes a minute).

### 5.9 Refused as root

```sh
sudo dotforge profile apply vm
echo $?
```

You see `don't run 'dotforge profile apply' as root or with sudo`, and
the exit code is **4**.

### 5.10 A failing step

Put a package that doesn't exist into a copy of the catalog entry for htop:

```sh
mkdir -p ~/.config/dotforge/catalog/applications
cat > ~/.config/dotforge/catalog/applications/htop.yml <<'YAML'
id: htop
name: htop
description: Broken on purpose
category: application
platforms: [linux]
installation:
  arch:
    packages: [htop-does-not-exist]
YAML
dotforge profile apply vm --yes
echo $?
rm ~/.config/dotforge/catalog/applications/htop.yml
```

pacman fails (`target not found`), the result shows `[FAILED ]` with the
last lines of pacman's error and the log path, and the exit code is **1**.
(Your own catalog entry replaces the built-in one, which is how this
test breaks it.)

### 5.11 Your own script

Add a script to the profile:

```sh
mkdir -p ~/.config/dotforge/profiles/vm/scripts
cat > ~/.config/dotforge/profiles/vm/scripts/hello.sh <<'SH'
#!/bin/bash
echo "hello from $(whoami) in $(pwd)" > ~/hello.txt
SH
echo 'scripts: [scripts/hello.sh]' >> ~/.config/dotforge/profiles/vm/profile.yml
dotforge profile apply vm --dry-run
```

The plan has a part `Your own scripts` with
`scripts/hello.sh: new, 2 line(s): you'll be asked before it runs`, and
`~/hello.txt` does not exist yet.

Now try `--yes`:

```sh
dotforge profile apply vm --yes
```

It still asks `Run your script scripts/hello.sh ...? [y/N]`, because
`--yes` never answers for scripts. Press Enter (no): the result shows it
as `[skipped]`. Run it again and type `y`:

```sh
dotforge profile apply vm
cat ~/hello.txt
```

You see `hello from arch in /home/arch/.config/dotforge/profiles/vm`.
Run apply once more: `Nothing to do`, the script doesn't run again. Edit
the script (change the text), and apply shows it as `changed` and asks
again.

Last, a script outside the profile folder is refused:

```sh
sed -i 's#scripts/hello.sh#../hello.sh#' ~/.config/dotforge/profiles/vm/profile.yml
dotforge profile validate vm
```

You see an error that `'../hello.sh' points outside the profile folder`.

When you're done, revert the VM to `clean-baseline` again.

## Milestone 6: config files and assets

These tests change files in your home folder, so run them in the
`arch-test` VM too. Revert it to `clean-baseline`, log in as `arch`, then:

```sh
cd ~/dotforge
git fetch && git checkout milestone-6-config-and-assets && git pull
cargo build --release
```

Make a profile with fastfetch (a small program that prints system info)
and install it:

```sh
mkdir -p ~/.config/dotforge/profiles/vm
cat > ~/.config/dotforge/profiles/vm/profile.yml <<'YAML'
format_version: 1
name: VM test
device:
  type: virtual-machine
  os:
    family: arch
    distribution: arch
applications:
  utilities: [fastfetch]
YAML
dotforge profile apply vm --yes
```

Short names used below:

```sh
P=~/.config/dotforge/profiles/vm
```

### 6.1 Link a config folder

Give fastfetch a tiny config in the profile:

```sh
mkdir -p $P/config/fastfetch
echo '{ "modules": ["title", "os", "kernel"] }' > $P/config/fastfetch/config.jsonc
dotforge profile apply vm --dry-run
```

The plan has a part `Config files and assets` with
`fastfetch: ~/.config/fastfetch -> config/fastfetch: link`. Nothing is
linked yet (`ls -l ~/.config/fastfetch` says no such file).

```sh
dotforge profile apply vm --yes
ls -l ~/.config/fastfetch
fastfetch
```

`ls -l` shows `~/.config/fastfetch -> /home/arch/.config/dotforge/profiles/vm/config/fastfetch`,
and fastfetch now prints only three lines: your user, the OS and the
kernel.

### 6.2 Editing through the link edits the profile

```sh
echo '{ "modules": ["title", "os", "kernel", "memory"] }' > ~/.config/fastfetch/config.jsonc
cat $P/config/fastfetch/config.jsonc
```

The profile's file changed too (it shows `memory`), because they are the
same file.

### 6.3 Your existing files are backed up, not overwritten

The VM already has a `~/.bashrc`. Put your own in the profile:

```sh
mkdir -p $P/config/home
cp ~/.bashrc $P/config/home/.bashrc
echo 'alias hi="echo hello from DotForge"' >> $P/config/home/.bashrc
dotforge profile apply vm
```

The plan says `~/.bashrc -> config/home/.bashrc: link; a file is there now
and will be moved to ~/.local/state/dotforge/backups/<date-time>/.bashrc`.
Type `y`. The result says `a file was there; moved it to ...`. Check:

```sh
ls ~/.local/state/dotforge/backups/*/
source ~/.bashrc && hi
```

The backup folder has your old `.bashrc`, and `hi` prints
`hello from DotForge`.

### 6.4 Applying again does nothing

```sh
dotforge profile apply vm
```

`This machine already matches the profile. Nothing to do.`

### 6.5 Removing something from the profile removes its link

```sh
mv $P/config/fastfetch /tmp/fastfetch-config
dotforge profile apply vm --dry-run
```

The plan says `~/.config/fastfetch: no longer in the profile; the link
will be removed`. Apply it with `--yes`, then `ls -l ~/.config/fastfetch`
says no such file: only the link was removed. Your file is safe in
`/tmp/fastfetch-config`. Put it back with
`mv /tmp/fastfetch-config $P/config/fastfetch` and apply again to re-link.

### 6.6 Mistakes in config/ are caught

```sh
mkdir -p $P/config/fastfech $P/config/kitty
dotforge profile validate vm
echo $?
```

You see an error `config/fastfech: there is no catalog entry 'fastfech';
did you mean 'fastfetch'?` and a warning `config/kitty: 'kitty' is not in
the profile, so this config is not linked`. The exit code is **4**, and
`dotforge profile apply vm` refuses to run. Clean up with
`rmdir $P/config/fastfech $P/config/kitty`.

### 6.7 Two things can't own the same place

```sh
mkdir -p $P/config/home/.config/fastfetch
echo '{}' > $P/config/home/.config/fastfetch/extra.jsonc
dotforge profile validate vm
rm -r $P/config/home/.config
```

You see an error that `'fastfetch' and '~/.config/fastfetch/extra.jsonc'
both want ...`, because `config/fastfetch/` already owns all of
`~/.config/fastfetch`.

### 6.8 A wallpaper

```sh
mkdir -p $P/assets/wallpapers
echo "pretend this is a picture" > $P/assets/wallpapers/sea.jpg
dotforge profile apply vm --yes
ls ~/.local/share/dotforge/wallpapers/
```

You see `sea.jpg`. Your configs (for example a wallpaper setting in
Hyprland) can always point at `~/.local/share/dotforge/wallpapers/`.

When you're done, revert the VM to `clean-baseline` again.
