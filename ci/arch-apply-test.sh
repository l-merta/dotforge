#!/usr/bin/env bash
# Real install test, run inside a throwaway Arch Linux container in CI.
# It applies a small profile for real (pacman, sudo, usermod), checks the
# result, applies again (nothing to do), then drops an app and removes it.
set -euo pipefail

BIN="$(pwd)/target/release/dotforge"
install -m 755 "$BIN" /usr/local/bin/dotforge

# DotForge refuses to run as root, so make a normal user with sudo.
useradd -m tester
echo 'tester ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/tester

as_tester() { sudo -u tester -H env USER=tester "$@"; }

profile_with() {
  as_tester mkdir -p /home/tester/.config/dotforge/profiles/ci
  as_tester tee /home/tester/.config/dotforge/profiles/ci/profile.yml > /dev/null <<YAML
format_version: 1
name: CI
device:
  type: desktop
  os:
    family: arch
    distribution: arch
applications:
  utilities: [$1]
YAML
}

step() { echo; echo "=== $* ==="; }
not_installed() { if pacman -Q "$1" 2>/dev/null; then echo "$1 should not be installed"; exit 1; fi; }

step "1. Dry run changes nothing"
profile_with "htop, fastfetch, zsh"
as_tester dotforge profile apply ci --dry-run
not_installed htop

step "2. Apply for real"
as_tester dotforge profile apply ci --yes
pacman -Q htop fastfetch zsh
getent passwd tester | grep -q ':/usr/bin/zsh$'
ls /home/tester/.local/state/dotforge/logs/

step "3. Applying again has nothing to do"
as_tester dotforge --json profile apply ci --yes | tee /tmp/second.json
grep -q '"result": "nothing_to_do"' /tmp/second.json

step "4. Drop htop: --yes keeps it"
profile_with "fastfetch, zsh"
as_tester dotforge profile apply ci --yes
pacman -Q htop

step "5. Answer 'r' to remove it"
echo r | as_tester dotforge profile apply ci
not_installed htop
pacman -Q fastfetch zsh

step "6. Running as root is refused"
set +e
dotforge profile apply ci --yes
code=$?
set -e
[ "$code" = 4 ]

echo
echo "All Arch apply checks passed."
