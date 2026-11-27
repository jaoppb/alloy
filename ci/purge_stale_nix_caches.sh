#!/usr/bin/env bash
# purge-stale-nix-caches — deletes Nix store cache entries that no job restores any more (#357, epic #154).
#
# Every `flake.nix`/`flake.lock`/`rust-toolchain.toml` change makes the nix-shell action save a new
# `nix-<os>-<entry>-<hash>` entry and leave the previous generation behind, so stale ones pile up against
# the repository's 10 GiB Actions cache limit. This keeps, on `main`, only entries ending in the current
# hash and drops the rest. It needs `actions: write`, so CI runs it only in a push-to-main job.
#
# Usage: ci/purge_stale_nix_caches.sh <current-flake-hash>   (needs GH_TOKEN; run inside a checkout)
set -euo pipefail

current_hash="${1:?usage: purge_stale_nix_caches.sh <current-flake-hash>}"

# Captured first so a failing `gh` call (missing permission, API error) fails the job under `set -e`.
cached_keys="$(gh cache list --ref refs/heads/main --key nix- --limit 100 --json key --jq '.[].key')"
stale_keys="$(printf '%s\n' "$cached_keys" | grep -vE -- "-${current_hash}\$" || true)"

printf '%s\n' "$stale_keys" | while read -r key; do
	[ -n "$key" ] || continue
	echo "deleting $key"
	gh cache delete "$key" --ref refs/heads/main
done
