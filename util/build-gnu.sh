#!/bin/bash

set -e

# Repository root (where util/ lives), captured before we cd into the GNU tree.
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Fetch upstream test suite if missing
mkdir -p findutils.gnu
if test ! -f findutils.gnu/README && ! tar xJf findutils.tar.xz --strip-components=1 -C findutils.gnu 2>/dev/null;then
    gnu_version=4.11.0
    b2=d3d881c60efb01eacf14a6bad95513b0186b694d0857a708bb28864fdf949fa99cbb80d2c8bf70e4bac65ebfba852bfa8435bf637fa7716e80e55a8b06ea1313
    echo "Could not find findutils.gnu/README. Downliading ${gnu_version}"
    curl -fsSL "https://ftp.gnu.org/gnu/findutils/findutils-${gnu_version}.tar.xz" -o findutils.tar.xz
    echo "${b2}"  findutils.tar.xz \
      | b2sum --check
    tar xJf findutils.tar.xz --strip-components=1 -C findutils.gnu
fi

# build the rust implementation
: ${PROFILE:=release}
cargo build --profile="${PROFILE}"
cp target/"${PROFILE}"/find ../findutils.gnu/find.rust
cp target/"${PROFILE}"/xargs ../findutils.gnu/xargs.rust

cd ../findutils.gnu
# Build upstream test suite
# needed for git src, unnecessary for tarball src
if test ! -f configure; then
    ./bootstrap
fi
if test ! -f Makefile; then
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
