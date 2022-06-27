# findutils

[![Crates.io](https://img.shields.io/crates/v/findutils.svg)](https://crates.io/crates/findutils)
[![dependency status](https://deps.rs/repo/github/uutils/findutils/status.svg)](https://deps.rs/repo/github/uutils/findutils)
[![codecov](https://codecov.io/gh/uutils/findutils/branch/master/graph/badge.svg)](https://codecov.io/gh/uutils/findutils)

Rust implementation of [GNU findutils](https://www.gnu.org/software/findutils/).

## Run the GNU testsuite on rust/findutils:

```
bash util/build-gnu.sh

# To run a specific test:
bash util/build-gnu.sh tests/misc/help-version.sh
```

## Comparing with GNU

![Evolution over time - GNU testsuite](https://github.com/uutils/findutils-tracking/blob/main/gnu-results.png?raw=true)
![Evolution over time - BFS testsuite](https://github.com/uutils/findutils-tracking/blob/main/bfs-results.png?raw=true)

For more details, see https://github.com/uutils/findutils-tracking/
