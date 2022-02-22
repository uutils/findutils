#!/bin/bash

set -eu

export LC_COLLATE=C

# Extract the failing test lines from log files
failing_tests() {
    sed -En 's/FAIL: ([^,:]*)[,:].*/\1/p' "$1"/{tests,{find,xargs}/testsuite}/*.log | sort
}

comm -3 <(failing_tests "$1") <(failing_tests "$2") | tr '\t' ',' | while IFS=, read old new foo; do
    if [ -n "$old" ]; then
        echo "::warning ::Congrats! The GNU test $old is now passing!"
    fi
    if [ -n "$new" ]; then
        echo "::error ::GNU test failed: $new. $new is passing on 'main'. Maybe you have to rebase?"
    fi
done
