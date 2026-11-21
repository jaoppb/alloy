#!/usr/bin/env bash
# verify-mold — proves the Linux artifacts of a build were linked by mold (issue #153, epic #154).
#
# `.cargo/config.toml` selects mold through `target.*.rustflags`, which a stray `RUSTFLAGS` env var
# (set by a tool such as cargo-fuzz or cargo-llvm-cov, or by a CI step) silently replaces. mold stamps
# its name in the ELF `.comment` section, so this script reads it back from every executable and
# shared object directly under each given directory (default: target/debug/deps) and fails on any
# that is not mold-linked, or when a directory holds no such file at all.
#
# Usage: ci/verify_mold.sh [artifact-dir ...]
set -euo pipefail

command -v readelf >/dev/null || {
	echo "readelf (binutils) is required to inspect the .comment section" >&2
	exit 1
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
if [ "$#" -eq 0 ]; then
	set -- "$target_dir/debug/deps"
fi

status=0
for artifact_dir in "$@"; do
	checked=0
	failed=0
	while IFS= read -r artifact; do
		readelf -h "$artifact" >/dev/null 2>&1 || continue
		checked=$((checked + 1))
		if ! readelf -p .comment "$artifact" 2>/dev/null | grep -q 'mold'; then
			echo "NOT linked with mold: $artifact" >&2
			failed=$((failed + 1))
		fi
	done < <(find "$artifact_dir" -maxdepth 1 -type f \( -perm -u+x -o -name '*.so' \))
	if [ "$checked" -eq 0 ]; then
		echo "no ELF artifact found under $artifact_dir — did the build run?" >&2
		status=1
		continue
	fi
	if [ "$failed" -gt 0 ]; then
		status=1
		continue
	fi
	echo "verify-mold: $checked artifact(s) under $artifact_dir linked with mold"
done
exit "$status"
