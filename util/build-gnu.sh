#!/bin/bash

set -e

# shellcheck source=util/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

GNU_DIR="${GNU_DIR:-$FINDUTILS_DIR/../findutils.gnu}"

if ! test -d "$GNU_DIR"; then
    echo "Could not find $GNU_DIR"
    echo "Set GNU_DIR or clone:"
    echo "  git clone https://git.savannah.gnu.org/git/findutils.git $GNU_DIR"
    exit 1
fi

# Build the Rust implementation
build_rust
cp "$FIND_BIN" "$GNU_DIR/find.rust"
cp "$XARGS_BIN" "$GNU_DIR/xargs.rust"

# Build upstream GNU findutils if needed
cd "$GNU_DIR"
if ! test -f configure; then
    ./bootstrap
    ./configure --quiet
    make -j "$(nproc)"
fi

# Overwrite the GNU versions with the Rust impl
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

# Collect results
PASS=0
SKIP=0
FAIL=0
XPASS=0
ERROR=0

LOG_FILE=./find/testsuite/find.log
if test -f "$LOG_FILE"; then
    ((PASS += $(sed -En 's/# of expected passes\s*//p' "$LOG_FILE"))) || :
    ((FAIL += $(sed -En 's/# of unexpected failures\s*//p' "$LOG_FILE"))) || :
fi

LOG_FILE=./xargs/testsuite/xargs.log
if test -f "$LOG_FILE"; then
    ((PASS += $(sed -En 's/# of expected passes\s*//p' "$LOG_FILE"))) || :
    ((FAIL += $(sed -En 's/# of unexpected failures\s*//p' "$LOG_FILE"))) || :
fi

((TOTAL = PASS + FAIL)) || :

LOG_FILE=./tests/test-suite.log
if test -f "$LOG_FILE"; then
    ((TOTAL += $(sed -n "s/.*# TOTAL: \(.*\)/\1/p"  "$LOG_FILE" | tr -d '\r' | head -n1))) || :
    ((PASS += $(sed -n "s/.*# PASS: \(.*\)/\1/p" "$LOG_FILE" | tr -d '\r' | head -n1))) || :
    ((SKIP += $(sed -n "s/.*# SKIP: \(.*\)/\1/p" "$LOG_FILE" | tr -d '\r' | head -n1))) || :
    ((FAIL += $(sed -n "s/.*# FAIL: \(.*\)/\1/p" "$LOG_FILE" | tr -d '\r' | head -n1))) || :
    ((XPASS += $(sed -n "s/.*# XPASS: \(.*\)/\1/p" "$LOG_FILE" | tr -d '\r' | head -n1))) || :
    ((ERROR += $(sed -n "s/.*# ERROR: \(.*\)/\1/p" "$LOG_FILE" | tr -d '\r' | head -n1))) || :
fi

check_total "$TOTAL"
print_summary "GNU tests" "$TOTAL" "$PASS" "$SKIP" "$FAIL" "$ERROR"
generate_result_json "${RESULT_FILE:-$GNU_DIR/../gnu-result.json}" "$TOTAL" "$PASS" "$SKIP" "$FAIL" "$XPASS" "$ERROR"
