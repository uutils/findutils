#!/bin/bash

set -eo pipefail

if ! test -d ../bfs; then
    echo "Could not find ../bfs"
    echo "git clone https://github.com/tavianator/bfs.git"
    exit 1
fi

# build the rust implementation
cargo build --release
FIND=$(readlink -f target/release/find)

cd ../bfs
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

# Default any missing numbers to zero (e.g. no tests skipped)
: ${PASS:=0}
: ${SKIP:=0}
: ${FAIL:=0}

TOTAL=$((PASS + SKIP + FAIL))
if (( TOTAL <= 1 )); then
    echo "Error in the execution, failing early"
    exit 1
fi

output="BFS tests summary = TOTAL: $TOTAL / PASS: $PASS / SKIP: $SKIP / FAIL: $FAIL"
echo "${output}"
if (( FAIL > 0 )); then echo "::warning ::${output}"; fi

jq -n \
   --arg date "$(date --rfc-email)" \
   --arg sha "$GITHUB_SHA" \
   --arg total "$TOTAL" \
   --arg pass "$PASS" \
   --arg skip "$SKIP" \
   --arg fail "$FAIL" \
   '{($date): { sha: $sha, total: $total, pass: $pass, skip: $skip, fail: $fail, }}' > ../bfs-result.json
