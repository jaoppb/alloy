#!/usr/bin/env bash
# hook-benchmark — blocking from v0.5 Phase P.
#
# Runs the `hook_overhead` criterion benchmark (round-trip `on_event` over a
# pre-compiled Rhai AST, PRD-001 N-01 target: p99 < 10us) and fails if the
# measured mean either:
#   1. exceeds the absolute PRD-001 N-01 ceiling, or
#   2. regressed more than `regression_factor` beyond the committed baseline
#      in core/runtime/rhai/benches/baselines/hook_overhead.json.
#
# `regression_factor` is deliberately generous (default 5x) rather than a tight
# statistical comparison against the previous run: this workspace has no
# persistent, dedicated benchmark runner, so CI-runner-to-CI-runner noise on a
# shared GitHub Actions machine can legitimately be 2-3x. A tight bound would
# be flaky; a loose one still catches a real algorithmic regression.
#
# Usage: ci/hook_benchmark.sh
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
baseline_file="$repo_root/core/runtime/rhai/benches/baselines/hook_overhead.json"
bench_name="on_event_overhead"

command -v jq >/dev/null || {
	echo "jq is required for the hook-benchmark gate" >&2
	exit 1
}

cargo bench -p rhai-runtime --bench hook_overhead

estimates_file="$repo_root/target/criterion/$bench_name/new/estimates.json"
test -f "$estimates_file" || {
	echo "expected criterion output at $estimates_file — did the bench name change?" >&2
	exit 1
}

mean_ns="$(jq -r '.mean.point_estimate' "$estimates_file")"
baseline_ns="$(jq -r '.mean_ns' "$baseline_file")"
regression_factor="$(jq -r '.regression_factor' "$baseline_file")"
absolute_ceiling_ns="$(jq -r '.absolute_ceiling_ns' "$baseline_file")"

echo "hook_overhead mean: ${mean_ns} ns  (baseline: ${baseline_ns} ns, ceiling: ${absolute_ceiling_ns} ns, regression factor: ${regression_factor}x)"

failed=0

if awk -v m="$mean_ns" -v c="$absolute_ceiling_ns" 'BEGIN { exit !(m > c) }'; then
	echo "✗ mean ${mean_ns} ns exceeds the PRD-001 N-01 ceiling of ${absolute_ceiling_ns} ns"
	failed=1
fi

if awk -v m="$mean_ns" -v b="$baseline_ns" -v f="$regression_factor" 'BEGIN { exit !(m > b * f) }'; then
	echo "✗ mean ${mean_ns} ns is more than ${regression_factor}x the committed baseline (${baseline_ns} ns) — regression"
	failed=1
fi

if [ "$failed" -ne 0 ]; then
	echo
	echo "hook-benchmark FAILED. If this is a deliberate, reviewed change to the hook dispatch path, update"
	echo "core/runtime/rhai/benches/baselines/hook_overhead.json's mean_ns to the new measured value."
	exit 1
fi

echo "✓ hook-benchmark passed"
