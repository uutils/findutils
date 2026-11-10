#!/bin/bash

set -e

# GNU findutils release the tests come from. The tarball is an unmodified
# copy in uutils/thirdparty-artifacts; to upgrade, publish the new release
# there, then update both values.
GNU_VERSION="4.11.0"
GNU_SHA256="bfd19cb06cc71f3352d567e90284d8cdac02ac89774bbeadf0b533b0c11432fd"

# Repository root (where util/ lives), captured before we cd into the GNU tree.
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Download the GNU tree unless one (e.g. a git clone) is already there.
if test ! -d ../findutils.gnu; then
    tarball="../findutils-${GNU_VERSION}.tar.xz"
    curl -fsSL -o "${tarball}" "https://github.com/uutils/thirdparty-artifacts/releases/download/findutils-${GNU_VERSION}/findutils-${GNU_VERSION}.tar.xz"
    echo "${GNU_SHA256}  ${tarball}" | sha256sum -c
    mkdir ../findutils.gnu
    tar -xJf "${tarball}" -C ../findutils.gnu --strip-components=1
fi

# build the rust implementation
: ${PROFILE:=release}
cargo build --profile="${PROFILE}"
cp target/"${PROFILE}"/find ../findutils.gnu/find.rust
cp target/"${PROFILE}"/xargs ../findutils.gnu/xargs.rust

# Clone and build upstream repo
cd ../findutils.gnu
# A git checkout needs bootstrapping; a release tarball ships configure.
if test ! -f configure; then
    ./bootstrap
fi
if test ! -f Makefile; then
    # Older gnulib does not build in C23 mode (default for newer GCC)
    ./configure --quiet CFLAGS="${CFLAGS:--O2} -std=gnu17"
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
