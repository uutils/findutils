#!/bin/bash

set -eo pipefail

# Repository root (where util/ lives), captured before we cd into the bfs tree.
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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

# Build a per-test JSON summary (name + status for every test) used by
# compare_test_results.py to detect per-test improvements/regressions.
RESULT_JSON="${RESULT_JSON:-../bfs-full-result.json}"
output="$(python3 "${REPO_DIR}/util/bfs_json_result.py" "$LOG_FILE" "${RESULT_JSON}")"
echo "${output}"

TOTAL=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['summary']['total'])" "${RESULT_JSON}")
FAIL=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['summary']['failed'])" "${RESULT_JSON}")

if (( TOTAL <= 1 )); then
    echo "Error in the execution, failing early"
    exit 1
fi

if (( FAIL > 0 )); then echo "::warning ::${output}"; fi
