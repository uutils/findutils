#!/bin/bash

set -e

if ! test -d ../bfs; then
    echo "Could not find ../bfs"
    echo "git clone https://github.com/tavianator/bfs.git"
    exit 1
fi

# build the rust implementation
cargo build --release
FIND=$(readlink -f target/release/find)

cd ../bfs
make -j "$(nproc)" all

# Run the GNU find compatibility tests by default
if test "$#" -eq 0; then
    set -- --verbose --gnu
fi

./tests.sh --bfs="$FIND" "$@"
