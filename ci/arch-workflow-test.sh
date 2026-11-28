#!/usr/bin/env bash
# Milestone 7: the complete workflow on a clean Arch Linux container, the
# way a new user would go through it, plus the ways it can go wrong:
# a partly installed machine, an invalid profile, a package that doesn't
# exist, a busy pacman, and a real AUR install with paru.
set -euo pipefail

BIN="$(pwd)/target/release/dotforge"
install -m 755 "$BIN" /usr/local/bin/dotforge

useradd -m tester
echo 'tester ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/tester
as_tester() { sudo -u tester -H env USER=tester "$@"; }
step() { echo; echo "=== $* ==="; }
expect_code() { # expect_code <code> <command...>
  local want=$1; shift
  set +e; "$@"; local got=$?; set -e
  if [ "$got" != "$want" ]; then echo "expected exit code $want, got $got"; exit 1; fi
}
not_installed() { if pacman -Q "$1" 2>/dev/null; then echo "$1 should not be installed"; exit 1; fi; }
packages_now() { pacman -Q | wc -l; }

P=/home/tester/.config/dotforge/profiles

step "0. An AUR helper, like a real Arch user has"
as_tester bash -c 'cd /tmp && git clone -q https://aur.archlinux.org/paru-bin.git && cd paru-bin && makepkg -si --noconfirm' > /dev/null
paru --version

step "1. Create a profile"
as_tester dotforge profile create desk --device desktop
as_tester dotforge profile list | grep -q desk

step "2. Fill it in: a desktop with apps, a setup action and an AUR package"
# 'yay' is not in the built-in catalog: add it as your own catalog entry.
as_tester mkdir -p /home/tester/.config/dotforge/catalog/applications
as_tester tee /home/tester/.config/dotforge/catalog/applications/yay.yml > /dev/null <<'YAML'
id: yay
name: yay
description: Another AUR helper (a small AUR package for testing)
category: application
platforms: [linux]
installation:
  arch:
    aur: [yay-bin]
YAML
as_tester tee $P/desk/profile.yml > /dev/null <<'YAML'
format_version: 1
name: Desk
device:
  type: desktop
  os:
    family: arch
    distribution: arch
applications:
  terminal: kitty
  utilities: [git, htop, yay]
  development: [docker]
rice:
  compositor: hyprland
  components:
    taskbar: waybar
YAML
as_tester dotforge profile validate desk
as_tester dotforge catalog validate

step "3. Status before: git is already there (a partly installed machine)"
as_tester dotforge profile status desk || true
as_tester dotforge --json profile status desk > /tmp/before.json || true
grep -q '"satisfied": false' /tmp/before.json

step "4. Dry run"
before=$(packages_now)
as_tester dotforge profile apply desk --dry-run | tee /tmp/plan.txt
grep -q "Already installed: git" /tmp/plan.txt
grep -q "Install from the AUR" /tmp/plan.txt
[ "$(packages_now)" = "$before" ]

step "5. Apply"
as_tester dotforge profile apply desk --yes
pacman -Q kitty htop hyprland waybar docker xdg-desktop-portal-hyprland yay-bin
id -nG tester | grep -qw docker
# No systemd in a container: enabling docker.service is skipped, not failed.

step "6. Status after, and applying again"
as_tester dotforge --json profile status desk > /tmp/after.json
grep -q '"satisfied": true' /tmp/after.json
before=$(packages_now)
as_tester dotforge profile apply desk --yes | tee /tmp/again.txt
[ "$(packages_now)" = "$before" ]

step "7. An invalid profile changes nothing"
as_tester cp -r $P/desk $P/broken
as_tester sed -i 's/htop/htpo/' $P/broken/profile.yml
before=$(packages_now)
expect_code 4 as_tester dotforge profile apply broken --yes
[ "$(packages_now)" = "$before" ]

step "8. A package that doesn't exist: honest failure, then a fixed re-run"
as_tester tee /home/tester/.config/dotforge/catalog/applications/btop.yml > /dev/null <<'YAML'
id: btop
name: btop
description: Broken on purpose
category: application
platforms: [linux]
installation:
  arch:
    packages: [btop-does-not-exist]
YAML
as_tester sed -i 's/utilities: \[git, htop, yay\]/utilities: [git, htop, yay, btop]/' $P/desk/profile.yml
expect_code 1 as_tester dotforge profile apply desk --yes
as_tester rm /home/tester/.config/dotforge/catalog/applications/btop.yml
as_tester dotforge profile apply desk --yes
pacman -Q btop

step "9. A busy (or interrupted) pacman is explained"
as_tester sed -i 's/utilities: \[git, htop, yay, btop\]/utilities: [git, htop, yay, btop, fastfetch]/' $P/desk/profile.yml
touch /var/lib/pacman/db.lck
expect_code 1 as_tester dotforge profile apply desk --yes
as_tester dotforge --json profile apply desk --yes > /tmp/busy.json || true
grep -q '"kind": "busy"' /tmp/busy.json
not_installed fastfetch
rm /var/lib/pacman/db.lck
as_tester dotforge profile apply desk --yes
pacman -Q fastfetch

echo
echo "The complete workflow passed."
