#!/usr/bin/env bash
# Record a progress line, commit everything, push and verify the remote.
# Usage: scripts/checkpoint.sh "<phase>" "<done>" "<next>"
set -euo pipefail

if [ $# -ne 3 ]; then
  echo "usage: $0 <phase> <done> <next>" >&2
  exit 2
fi

phase=$1 done=$2 next=$3
root=$(git rev-parse --show-toplevel)
branch=$(git branch --show-current)

if [ "$branch" = "main" ] || [ "$branch" = "master" ]; then
  echo "refusing to checkpoint on $branch" >&2
  exit 1
fi

printf '| %s | %s | %s | %s |\n' "$(date -u '+%Y-%m-%d %H:%M')" "$phase" "$done" "$next" >> "$root/docs/v1/PROGRESS.md"

git -C "$root" add -A
msg=(-m "checkpoint($phase): $done" -m "Next: $next")
# Optional trailer lines (for example Co-Authored-By) from the environment.
if [ -n "${CHECKPOINT_TRAILER:-}" ]; then msg+=(-m "$CHECKPOINT_TRAILER"); fi
git -C "$root" commit -q "${msg[@]}"

ok=0
for wait in 2 4 8; do
  if git -C "$root" push -q -u origin "$branch"; then ok=1; break; fi
  echo "push failed, retrying in ${wait}s" >&2
  sleep "$wait"
done
if [ $ok -ne 1 ]; then
  echo "push failed: work is saved only in the local repository" >&2
  exit 1
fi

local_sha=$(git -C "$root" rev-parse HEAD)
remote_sha=$(git -C "$root" ls-remote origin "refs/heads/$branch" | cut -f1)
if [ "$local_sha" != "$remote_sha" ]; then
  echo "remote $branch is at $remote_sha, expected $local_sha" >&2
  exit 1
fi
echo "checkpoint pushed: $local_sha"
