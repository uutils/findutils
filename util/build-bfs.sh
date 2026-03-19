#!/bin/bash

set -eo pipefail

# shellcheck source=util/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

if test -z "$BFS_DIR"; then
    if test -d "$FINDUTILS_DIR/../bfs"; then
        BFS_DIR="$FINDUTILS_DIR/../bfs"
    elif test -d "$FINDUTILS_DIR/../../tavianator/bfs"; then
        BFS_DIR="$FINDUTILS_DIR/../../tavianator/bfs"
    else
        echo "Could not find bfs checkout"
        echo "Set BFS_DIR or clone:"
        echo "  git clone https://github.com/tavianator/bfs.git $FINDUTILS_DIR/../bfs"
        exit 1
    fi
fi

# Build the Rust implementation
build_rust
FIND="$FIND_BIN"

cd "$BFS_DIR"
./configure NOLIBS=y
make -j "$(nproc)" bin/tests/{mksock,xtouch}

# Run the GNU find compatibility tests by default
if test "$#" -eq 0; then
    set -- --verbose=tests --gnu --sudo
fi

LOG_FILE=tests.log
./tests/tests.sh --bfs="$FIND" "$@" 2>&1 | tee "$LOG_FILE" || :

PASS=$(sed -En 's|^\[PASS] *([0-9]+) / .*|\1|p' "$LOG_FILE")
SKIP=$(sed -En 's|^\[SKIP] *([0-9]+) / .*|\1|p' "$LOG_FILE")
FAIL=$(sed -En 's|^\[FAIL] *([0-9]+) / .*|\1|p' "$LOG_FILE")

: "${PASS:=0}"
: "${SKIP:=0}"
: "${FAIL:=0}"

TOTAL=$((PASS + SKIP + FAIL))

check_total "$TOTAL"
print_summary "BFS tests" "$TOTAL" "$PASS" "$SKIP" "$FAIL"
generate_result_json "${RESULT_FILE:-$BFS_DIR/../bfs-result.json}" "$TOTAL" "$PASS" "$SKIP" "$FAIL"
