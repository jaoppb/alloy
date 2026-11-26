#!/usr/bin/env bash
# purge-stale-nix-caches — deletes Nix store cache entries that no job restores any more (#357, epic #154).
#
# Every `flake.nix`/`flake.lock`/`rust-toolchain.toml` change makes the nix-shell action save a new
# `nix-<os>-<entry>-<hash>` entry and leave the previous generation behind, so stale ones pile up against
# the repository's 10 GiB Actions cache limit. This keeps, on `main`, only entries ending in the current
# hash and drops the rest, plus the per-shell Linux entries (rust, coverage, unsafe-audit, fuzz) that the
# shared `toolchain` entry replaced. It needs `actions: write`, so CI runs it only in a push-to-main job.
#
# Usage: ci/purge_stale_nix_caches.sh <current-flake-hash>   (needs GH_TOKEN and GH_REPO or a checkout)
set -euo pipefail

current_hash="${1:?usage: purge_stale_nix_caches.sh <current-flake-hash>}"

stale_keys="$(
	gh cache list --ref refs/heads/main --key nix- --limit 100 --json key --jq '.[].key' |
		grep -vE -- "-${current_hash}\$" || true
)"
replaced_keys="$(
	gh cache list --ref refs/heads/main --key nix-Linux- --limit 100 --json key --jq '.[].key' |
		grep -E -- '^nix-Linux-(rust|coverage|unsafe-audit|fuzz)-' || true
)"

printf '%s\n%s\n' "$stale_keys" "$replaced_keys" | sort -u | while read -r key; do
	[ -n "$key" ] || continue
	echo "deleting $key"
	gh cache delete "$key" --ref refs/heads/main
done
