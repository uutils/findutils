# 📦 Findutils 0.1.0 Release

Findutils 0.1.0 (tagged 2021-03-15) is the first tagged release of the project.
It has no GitHub release page, so these notes were reconstructed from the git
history up to the `0.1.0` tag.

It covers the initial implementation of `find`: the matcher-tree architecture,
the bulk of the GNU predicates, and the test infrastructure the project still
uses today. `xargs` did not exist yet — it arrived in 0.3.0.

## Highlights

### The `find` expression engine
* Initial implementation supporting `-name`, `-print` and a subset of `-type`,
  by @mcharsley
* Logical operators: `-o`/`-or`, `-a`/`-and`, `!`, parentheses, and comma lists
* `-depth`, `-maxdepth`, `-mindepth` and depth-first traversal
* `-prune`
* Directory walking switched to the `walkdir` crate

### Predicates
* `-size` by @mcharsley
* `-newer`, `-ctime`, `-atime`, `-mtime` by @mcharsley
* `-exec` and `-execdir` by @mcharsley, plus a fix for `-execdir` on the root
  directory
* `-perm` by @mcharsley
* `-type`: added `l`, `b`, `c`, `p` and `s` by @ilius
* `-delete` by @Arcterus, extended to all file types and made to skip the
  current directory; tests and fixes by @dahc

### Correctness fixes
* `-help` was matched too aggressively
* A lone `-` is treated as a filename
* Double negation (`! !`) fixed by @Arcterus

### Portability
* Windows support: path separator handling in tests, `testing-commandline`
  argument reporting, and `#[cfg(unix)]` gating of the permission code, by
  @mcharsley
* AppVeyor (Windows) and Travis CI configurations by Lei Zhang

### Testing & infrastructure
* Fake-stdout dependency injection so tests can assert on exact output, and
  argument-parsing tests, by @mcharsley
* Integration tests for the `find` command by @dahc
* GitHub Actions CI workflow by @dahc
* Code coverage via Codecov and tarpaulin by @Arcterus
* Migration to the Rust 2018 edition and `rustfmt` formatting by @Arcterus
* Various clippy fixes, including `clippy::borrowed_box` and
  `clippy::unnecessary_wraps` on `process_dir`, by @ilius

### Dependencies
Regular dependency bumps via Dependabot: `regex` (0.2.2 → 1.4.5), `walkdir`
(2.2.7 → 2.3.1), `glob` (0.2.11 → 0.3.0), `tempdir` (0.3.5 → 0.3.7),
`assert_cmd` (1.0.2 → 1.0.3) and `predicates` (1.0.6 → 1.0.7).

## Contributors
@mcharsley, @Arcterus, @dahc, @ilius, @sylvestre, @rofrol, @bippityboppity,
Lei Zhang, Jeremy Soller and cnd.
