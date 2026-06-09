#!/bin/bash

set -e

# Repository root (where util/ lives), captured before we cd into the GNU tree.
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if test ! -d ../findutils.gnu; then
    echo "Could not find ../findutils.gnu"
    echo "git clone https://git.savannah.gnu.org/git/findutils.git findutils.gnu"
    exit 1
fi

# build the rust implementation
: ${PROFILE:=release}
cargo build --profile="${PROFILE}"
cp target/"${PROFILE}"/find ../findutils.gnu/find.rust
cp target/"${PROFILE}"/xargs ../findutils.gnu/xargs.rust

# Clone and build upstream repo
cd ../findutils.gnu
if test ! -f configure; then
    ./bootstrap
    ./configure --quiet
    make -j "$(nproc)"
fi

# overwrite the GNU version with the rust impl
cp find.rust find/find
cp xargs.rust xargs/xargs

if test -n "$1"; then
    # if set, run only the test passed
    export RUN_TEST="TESTS=$1"
fi

# Run the tests
make check-TESTS $RUN_TEST || :
make -C find/testsuite check || :
make -C xargs/testsuite check || :

# Build a per-test JSON summary (name + status for every test) used by
# compare_test_results.py to detect per-test improvements/regressions.
RESULT_JSON="${RESULT_JSON:-../findutils-gnu-full-result.json}"
output="$(python3 "${REPO_DIR}/util/gnu_json_result.py" . "${RESULT_JSON}")"
echo "${output}"

TOTAL=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['summary']['total'])" "${RESULT_JSON}")
FAIL=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['summary']['failed'])" "${RESULT_JSON}")

if ((TOTAL <= 1)); then
    echo "Error in the execution, failing early"
    exit 1
fi

if [[ "$FAIL" -gt 0 ]]; then echo "::warning ::${output}"; fi
