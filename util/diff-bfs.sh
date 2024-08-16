#!/bin/bash

set -eu

export LC_COLLATE=C

# Extract the failing test lines from log files
failing_tests() {
    sed -En 's/^\[FAIL\] (.*[a-z].*)/\1/p' "$1" | sort
}

comm -3 <(failing_tests "$1") <(failing_tests "$2") | tr '\t' ',' | while IFS=, read old new; do
    if [ -n "$old" ]; then
        echo "::warning ::Congrats! The bfs test $old is now passing!"
    fi
    if [ -n "$new" ]; then
        echo "::error ::bfs test failed: $new. $new is passing on 'main'. Maybe you have to rebase?"
    fi
done
