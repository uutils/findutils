# 📦 Findutils 0.9.1 Release

Findutils 0.9.1 is a robustness and portability release. It follows up quickly on the 0.9.0 milestone (which introduced `locate`/`updatedb`) by hardening the existing utilities against a series of panics on malformed or unusual input, adding **WebAssembly (`wasm32-wasip1`) build support**, shipping **statically linked musl binaries** for Linux, and reworking how we track GNU/bfs test-suite compatibility in CI.

This release saw the first contribution from @leeewee, who landed most of the panic fixes below.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

## GNU Test Suite Compatibility

Starting with this release, compatibility is tracked **per individual test** (PASS/FAIL/SKIP by test name) rather than by an aggregate count — see “Per-test compatibility tracking” below. Under the new per-test accounting, 0.9.1 stands at:

| Result | find (GNU suite) | bfs suite |
|--------|------------------|-----------|
| Pass   | 416              | 267       |
| Fail   | 78               | 40        |
| Skip   | 1                | 6         |
| Total  | 495              | 313       |

0.9.1 is primarily a robustness/infrastructure release, so `find`/`xargs` matching semantics are essentially unchanged from 0.9.0; the panic fixes harden edge cases (malformed brackets, multibyte escapes, unmapped UIDs, short databases) rather than altering option behavior.

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## Highlights

### Robustness — no more panics on malformed input
* `find -ls`: don't panic on an unmapped owner UID/GID by @leeewee in https://github.com/uutils/findutils/pull/721
* `find -name`: don't panic on a malformed POSIX bracket class by @leeewee in https://github.com/uutils/findutils/pull/722
* `find -printf`: don't panic on a multibyte char after an octal escape by @leeewee in https://github.com/uutils/findutils/pull/723
* `find -printf`: correctly test an octal escape before a multibyte char by @leeewee in https://github.com/uutils/findutils/pull/727
* `find -printf`: reject an over-large field width instead of panicking by @leeewee in https://github.com/uutils/findutils/pull/734
* `xargs`: do not panic on empty input in replace (`-I`) mode by @leeewee in https://github.com/uutils/findutils/pull/736
* `locate`: don't panic on a too-short `--database` file by @leeewee in https://github.com/uutils/findutils/pull/718

### Portability: WebAssembly and static musl binaries
* Support compiling for non-Unix targets such as wasm, and build + lint the `wasm32-wasip1` target in CI by @sylvestre in https://github.com/uutils/findutils/pull/725
* Build statically linked musl binaries for `x86_64` and `aarch64` by @sylvestre in https://github.com/uutils/findutils/pull/741

### xargs
* Clarify the `-x` and `-t` help text by @jmr in https://github.com/uutils/findutils/pull/708

### Project, CI & release
* Per-test GNU/bfs comparison and PR comment, modeled on the uutils sed/grep `GnuTests` workflow by @sylvestre in https://github.com/uutils/findutils/pull/726
* Define the `[profile.dist]` used by cargo-dist by @sylvestre in https://github.com/uutils/findutils/pull/739
* Inherit `lto = "thin"` in `Cargo.toml` by @oech3 in https://github.com/uutils/findutils/pull/740
* Add 2 missing binaries to `latest-commit` by @oech3 in https://github.com/uutils/findutils/pull/743
* `CONTRIBUTING.md`: replace coreutils references with findutils by @oech3 in https://github.com/uutils/findutils/pull/738

### Dependencies
Dependency and GitHub Action bumps via Dependabot and a manual crate refresh (@xtqqczze, #729): `itertools` (0.14.0 → 0.15.0, #742), `regex` (1.12.3 → 1.12.4, #728), `cargo-dist` (0.28.0 → 0.32.0, #748), `actions/checkout` (4 → 6 → 7, #711/#744), `codecov/codecov-action` (6 → 7, #712), and `moonrepo/setup-rust` (0 → 1, #709).

## New Contributors
* @leeewee made their first contribution in https://github.com/uutils/findutils/pull/718

**Full Changelog**: https://github.com/uutils/findutils/compare/0.9.0...0.9.1


## Install findutils 0.9.1

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/uutils/findutils/releases/download/0.9.1/findutils-installer.sh | sh
```

## Download findutils 0.9.1

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux (glibc) | [checksum](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |
| [findutils-x86_64-unknown-linux-musl.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-unknown-linux-musl.tar.xz) | x64 Linux (static musl) | [checksum](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-x86_64-unknown-linux-musl.tar.xz.sha256) |
| [findutils-aarch64-unknown-linux-musl.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-aarch64-unknown-linux-musl.tar.xz) | ARM64 Linux (static musl) | [checksum](https://github.com/uutils/findutils/releases/download/0.9.1/findutils-aarch64-unknown-linux-musl.tar.xz.sha256) |
