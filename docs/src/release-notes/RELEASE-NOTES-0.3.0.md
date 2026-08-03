# 📦 Findutils 0.3.0 Release

Findutils 0.3.0 (tagged 2022-01-26) has no GitHub release page, so these notes
were reconstructed from the git history between the `0.1.0` and `0.3.0` tags.
The version was bumped to 0.2.0 during this period, but no `0.2.0` tag or
release was ever published — so this entry covers everything since 0.1.0.

The headline change is the **first implementation of `xargs`**, which makes the
project a two-utility suite. This release also adds `-regex`, `-print0` and
GNU-compatible `-printf`, and sets up the compatibility testing against the GNU
and bfs test suites that the project still relies on.

## Highlights

### xargs
* An initial implementation of `xargs` by @refi64 in
  https://github.com/uutils/findutils/pull/121

### find: new features
* GNU-compatible `-printf` by @refi64 in
  https://github.com/uutils/findutils/pull/120
* `-regex` matching by @refi64 in
  https://github.com/uutils/findutils/pull/126
* `-print0` by @refi64 in
  https://github.com/uutils/findutils/pull/123
* `--version` on the command line by @sylvestre

### find: fixes
* Make multi-`-exec` only match `{} +` with nothing in between by @tavianator
  in https://github.com/uutils/findutils/pull/91
* `-exec`: handle parent directories more carefully by @tavianator in
  https://github.com/uutils/findutils/pull/92

### Compatibility testing
* Automated compatibility tests, in two parts, by @refi64 in
  https://github.com/uutils/findutils/pull/129 and
  https://github.com/uutils/findutils/pull/130
* A script to run the GNU findutils test suite, run it in CI, allow running a
  single test, avoid rebuilding GNU every time, and document the workflow, by
  @sylvestre
* Run the bfs test suite in CI by @tavianator

### Project & infrastructure
* Code coverage restored via Codecov by @sylvestre
* `tempdir` replaced by `tempfile`, and `tempfile` moved to dev-dependencies
  (`cargo udeps`) by @sylvestre
* `uucore` updated to 0.0.12 by @refi64
* Migration to GitHub-native Dependabot
* Clippy cleanups, README updates (crates.io badge, Travis badge removed) and
  removal of the unused `AUTHORS` file by @sylvestre

## Contributors
@refi64, @sylvestre and @tavianator.
