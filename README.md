# findutils

[![Crates.io](https://img.shields.io/crates/v/findutils.svg)](https://crates.io/crates/findutils)
[![Discord](https://img.shields.io/badge/discord-join-7289DA.svg?logo=discord&longCache=true&style=flat)](https://discord.gg/wQVJbvJ)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/findutils/blob/main/LICENSE)
[![dependency status](https://deps.rs/repo/github/uutils/findutils/status.svg)](https://deps.rs/repo/github/uutils/findutils)
[![codecov](https://codecov.io/gh/uutils/findutils/branch/master/graph/badge.svg)](https://codecov.io/gh/uutils/findutils)

Rust implementation of [GNU findutils](https://www.gnu.org/software/findutils/): `xargs`, `find`, `locate` and `updatedb`.
The goal is to be a full drop-in replacement of the original commands.

## Run the GNU testsuite on rust/findutils:

```
bash util/build-gnu.sh

# To run a specific test:
bash util/build-gnu.sh tests/misc/help-version.sh
```

## Comparing with GNU

![Evolution over time - GNU testsuite](https://github.com/uutils/findutils-tracking/blob/main/gnu-results.png?raw=true)
![Evolution over time - BFS testsuite](https://github.com/uutils/findutils-tracking/blob/main/bfs-results.png?raw=true)

## Build/run with BFS

[bfs](https://github.com/tavianator/bfs) is a variant of the UNIX find command that operates breadth-first rather than depth-first.

```
bash util/build-bfs.sh

# To run a specific test:
bash util/build-bfs.sh posix/basic
```

For more details, see https://github.com/uutils/findutils-tracking/
