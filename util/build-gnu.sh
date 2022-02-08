#!/bin/bash

set -e

if test ! -d ../findutils.gnu; then
    echo "Could not find ../findutils.gnu"
    echo "git clone https://git.savannah.gnu.org/git/findutils.git findutils.gnu"
    exit 1
fi

# build the rust implementation
cargo build --release
cp target/release/find ../findutils.gnu/find.rust
cp target/release/xargs ../findutils.gnu/xargs.rust

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

if ((TOTAL <= 1)); then
    echo "Error in the execution, failing early"
    exit 1
fi

output="GNU tests summary = TOTAL: $TOTAL / PASS: $PASS / FAIL: $FAIL / ERROR: $ERROR"
echo "${output}"
if [[ "$FAIL" -gt 0 || "$ERROR" -gt 0 ]]; then echo "::warning ::${output}" ; fi
jq -n \
   --arg date "$(date --rfc-email)" \
   --arg sha "$GITHUB_SHA" \
   --arg total "$TOTAL" \
   --arg pass "$PASS" \
   --arg skip "$SKIP" \
   --arg fail "$FAIL" \
   --arg xpass "$XPASS" \
   --arg error "$ERROR" \
   '{($date): { sha: $sha, total: $total, pass: $pass, skip: $skip, fail: $fail, xpass: $xpass, error: $error, }}' > ../gnu-result.json
