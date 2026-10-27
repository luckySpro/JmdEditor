#!/bin/sh
# Starts jmd-server with the settings in jmd-server.env. Used as the start
# command in BaoTa (宝塔), Supervisor or systemd.
cd "$(dirname "$0")" || exit 1
# The package ships only the example, so an upgrade never overwrites your settings.
[ -f jmd-server.env ] || cp jmd-server.env.example jmd-server.env
if [ -f jmd-server.env ]; then
  set -a
  . ./jmd-server.env
  set +a
fi
exec ./jmd-server
