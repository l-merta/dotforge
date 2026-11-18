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
rm -rf /tmp/dotforge-test /tmp/broken
unset DOTFORGE_CONFIG_DIR
```

---

## Milestone 1: catalog, validation, `--json`, exit codes

### 1.1 Browse the catalog

```sh
dotforge catalog list
```

You see `Catalog entries: 4`, then the categories. Hyprland and Niri
are under Compositors, Noctalia under Shells, Kitty under Applications.
Empty categories say `(none yet)`.

### 1.2 Filter by category

```sh
dotforge catalog list compositor
```

Only Hyprland and Niri are shown.

### 1.3 Look at one entry

```sh
dotforge catalog show hyprland
```

You see Hyprland's ID, category, description, `Source: built-in
(compositors/hyprland.yml)`, capabilities, dependencies, packages and
config folder.

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

You see `0 error(s), 2 warning(s)`. The warnings say that Hyprland's
two `xdg-desktop-portal` dependencies are not catalog entries yet. The
last line is `The catalog is valid.` and the exit code is **0**.

### 1.6 JSON output

```sh
dotforge --json catalog show kitty
dotforge catalog show kitty --json
```

Both print the same JSON. It starts with `"data": {` and ends with
`"exit_code": 0` and `"ok": true`.

### 1.7 Check a profile that has problems

```sh
dotforge profile validate ./profiles/test
echo $?
```

You see `17 error(s)`, one line for each app or component that isn't in
the catalog yet (`'steam' is not in the catalog`, ...). The exit code is
**4** (invalid). This is expected for now.

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
