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

step "6. A custom script runs only after its own yes"
as_tester mkdir -p /home/tester/.config/dotforge/profiles/ci/scripts
as_tester tee /home/tester/.config/dotforge/profiles/ci/scripts/hello.sh > /dev/null <<'SH'
#!/bin/bash
echo "hello from $(whoami)" > ~/script-ran
SH
echo 'scripts: [scripts/hello.sh]' | as_tester tee -a /home/tester/.config/dotforge/profiles/ci/profile.yml > /dev/null
as_tester dotforge profile apply ci --yes < /dev/null   # --yes doesn't approve scripts
[ ! -e /home/tester/script-ran ]
echo y | as_tester dotforge profile apply ci
grep -q "hello from tester" /home/tester/script-ran

step "7. Config files are linked, and what was there is backed up"
P=/home/tester/.config/dotforge/profiles/ci
as_tester mkdir -p $P/config/fastfetch $P/config/home /home/tester/.config/fastfetch
echo '{"modules": ["os"]}' | as_tester tee $P/config/fastfetch/config.jsonc > /dev/null
echo 'alias ll="ls -l"' | as_tester tee $P/config/home/.zshrc > /dev/null
echo 'old' | as_tester tee /home/tester/.config/fastfetch/config.jsonc > /dev/null
as_tester dotforge profile apply ci --yes < /dev/null
[ "$(readlink /home/tester/.config/fastfetch)" = "$P/config/fastfetch" ]
[ "$(readlink /home/tester/.zshrc)" = "$P/config/home/.zshrc" ]
grep -rq old /home/tester/.local/state/dotforge/backups/
as_tester dotforge profile apply ci --yes < /dev/null | grep -q "Nothing to do"

step "8. Running as root is refused"
set +e
dotforge profile apply ci --yes
code=$?
set -e
[ "$code" = 4 ]

echo
echo "All Arch apply checks passed."
