#!/bin/bash
# Common utilities for findutils compatibility test scripts.

COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FINDUTILS_DIR="$(cd "$COMMON_DIR/.." && pwd)"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$FINDUTILS_DIR/target}"

FIND_BIN="$CARGO_TARGET_DIR/release/find"
XARGS_BIN="$CARGO_TARGET_DIR/release/xargs"

build_rust() {
    echo "Building Rust findutils..."
    (cd "$FINDUTILS_DIR" && cargo build --release)
}

check_total() {
    local total="$1"
    if (( total <= 1 )); then
        echo "Error in the execution, failing early"
        exit 1
    fi
}

print_summary() {
    local label="$1" total="$2" pass="$3" skip="$4" fail="$5" error="${6:-0}"
    local output="$label summary = TOTAL: $total / PASS: $pass / SKIP: $skip / FAIL: $fail / ERROR: $error"
    echo "$output"
    if (( fail > 0 || error > 0 )); then
        echo "::warning ::$output"
    fi
}

generate_result_json() {
    local output_file="$1" total="$2" pass="$3" skip="$4" fail="$5"
    local xpass="${6:-0}" error="${7:-0}"
    jq -n \
        --arg date "$(date --rfc-email 2>/dev/null || date '+%a, %d %b %Y %T %z')" \
        --arg sha "${GITHUB_SHA:-$(git -C "$FINDUTILS_DIR" rev-parse HEAD 2>/dev/null || echo unknown)}" \
        --arg total "$total" \
        --arg pass "$pass" \
        --arg skip "$skip" \
        --arg fail "$fail" \
        --arg xpass "$xpass" \
        --arg error "$error" \
        '{($date): { sha: $sha, total: $total, pass: $pass, skip: $skip, fail: $fail, xpass: $xpass, error: $error, }}' > "$output_file"
}
