#!/usr/bin/env bash
# unsafe-audit (ADR-0018) — blocking from v0.5 Phase P.
#
# Two checks:
#   1. `cargo geiger --forbid-only` — every hand-written crate in this workspace
#      keeps `#![forbid(unsafe_code)]`, no exception (ADR-0018, first paragraph).
#   2. A full `cargo-geiger` scan (JSON output), reduced to the workspace's
#      *direct* third-party dependencies (from `cargo metadata`) — the reviewed
#      surface `unsafe-allowlist.toml` is meant to track. A direct dependency
#      carrying `unsafe` that is not in the allowlist fails the gate.
#
# Scoped to direct dependencies, not the whole transitive graph: a direct
# `Cargo.toml` addition is a reviewed event; a crate five layers down pulled in
# by something like `clap`/`tracing`/`regex` (memchr, smallvec, once_cell,
# arrayvec, thread_local, sharded-slab, regex-automata, …) is ecosystem-standard
# unsafe this project never chose individually and cannot review one entry at a
# time without the allowlist becoming noise nobody reads. If a transitive
# dependency's `unsafe` ever needs its own scrutiny (a security report, a
# licence change), promote it to a direct dependency or name it explicitly here.
#
# Usage: ci/unsafe_audit.sh [path/to/manifest/Cargo.toml]
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest_path="${1:-$repo_root/alloy/Cargo.toml}"
allowlist="$repo_root/unsafe-allowlist.toml"

command -v cargo-geiger >/dev/null || cargo install cargo-geiger --locked
command -v jq >/dev/null || {
	echo "jq is required for the unsafe-audit gate" >&2
	exit 1
}

echo "== forbid(unsafe_code) sweep =="
cargo geiger --manifest-path "$manifest_path" --forbid-only --output-format Ascii

test -s "$allowlist" && grep -q '\[\[allow\]\]' "$allowlist"
echo "✓ unsafe-allowlist.toml present and non-empty"

echo
echo "== direct-dependency unsafe scan =="
geiger_json="$(mktemp)"
trap 'rm -f "$geiger_json"' EXIT
cargo geiger --manifest-path "$manifest_path" --output-format Json -q >"$geiger_json"

direct_deps="$(
	cargo metadata --format-version 1 --no-deps |
		jq -r '[.packages[].dependencies[] | select(.kind == null) | .name] | unique[]'
)"

allowed_crates="$(grep -oP '^crate = "\K[^"]+' "$allowlist" || true)"

failures=0
while IFS= read -r dep; do
	[ -z "$dep" ] && continue
	# Skip workspace-internal path dependencies — they carry no version entry
	# in the geiger report under this name if they are not on crates.io, and
	# they are `#![forbid(unsafe_code)]` by construction (arch-lint / CI).
	is_workspace_member=false
	for member in engine dom css html graphics network window rhai-runtime rhai-bindings devtools extension js alloy; do
		[ "$dep" = "$member" ] && is_workspace_member=true && break
	done
	$is_workspace_member && continue

	unsafe_total="$(
		jq -r --arg name "$dep" '
            [.packages[] | select(.package.id.name == $name) |
             (.unsafety.used.functions.unsafe_ + .unsafety.used.exprs.unsafe_ +
              .unsafety.used.item_impls.unsafe_ + .unsafety.used.item_traits.unsafe_ +
              .unsafety.used.methods.unsafe_)] | add // 0
        ' "$geiger_json"
	)"

	if [ "$unsafe_total" -gt 0 ]; then
		if ! grep -qx "$dep" <<<"$allowed_crates"; then
			echo "✗ direct dependency '$dep' carries $unsafe_total unsafe item(s) and is not in unsafe-allowlist.toml"
			failures=$((failures + 1))
		else
			echo "✓ '$dep' ($unsafe_total unsafe item(s)) — allowlisted"
		fi
	fi
done <<<"$direct_deps"

if [ "$failures" -gt 0 ]; then
	echo
	echo "unsafe-audit FAILED: $failures direct dependency/dependencies need a reviewed unsafe-allowlist.toml entry (ADR-0018)."
	exit 1
fi

echo
echo "✓ unsafe-audit passed — every direct dependency carrying unsafe is reviewed in unsafe-allowlist.toml"
