#!/usr/bin/env bash
# Packs jmd-server for a plain Linux server (BaoTa/宝塔, Supervisor, systemd):
# the static binary, start script, config template, fonts and browser build.
#
#   scripts/package-server.sh <jmd-server binary> <out.tar.gz>
#
# Expects apps/desktop/dist (pnpm build) to exist; fonts are fetched if missing.
set -euo pipefail

bin="$1"
out="$2"
root="$(cd "$(dirname "$0")/.." && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
pkg="$stage/jmd-server"
mkdir -p "$pkg"

install -m 755 "$bin" "$pkg/jmd-server"
install -m 755 "$root/deploy/linux/start.sh" "$pkg/start.sh"
install -m 644 "$root/deploy/linux/jmd-server.env.example" "$root/deploy/linux/jmd-server.service" "$root/deploy/linux/宝塔部署说明.md" "$pkg/"
"$root/scripts/fetch-fonts.sh" "$pkg/fonts" >/dev/null
cp -R "$root/apps/desktop/dist" "$pkg/web"

tar -C "$stage" -czf "$out" jmd-server
echo "packed $out ($(du -h "$out" | cut -f1))"
