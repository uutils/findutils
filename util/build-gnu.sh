#!/bin/bash

set -e
if test ! -d ../findutils.gnu; then
    echo "Could not find ../findutils.gnu"
    echo "git clone  https://git.savannah.gnu.org/git/findutils.git findutils.gnu"
    exit 1
fi

# build the rust implementation
cargo build --release
cp target/release/find ../findutils.gnu/find.rust

# Clone and build upstream repo
cd ../findutils.gnu
if test ! -f configure; then
    ./bootstrap
    ./configure --quiet --disable-gcc-warnings
    make -j "$(nproc)"
fi

# overwrite the GNU version with the rust impl
cp find.rust find/find

# Run the tests
make check-TESTS
