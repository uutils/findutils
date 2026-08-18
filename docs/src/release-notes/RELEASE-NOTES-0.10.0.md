# 📦 Findutils 0.10.0 Release

Findutils 0.10.0 is a GNU-compatibility and robustness release. It sharpens several `find` behaviors that silently differed from GNU (`-printf %T+` formatting, `-newerXY` predicate validation, `--files0-from` input validation), fixes an `xargs` bug where the default command line size could make `execvp()` fail with `E2BIG`, improves `xargs` length accounting on Windows, and continues the campaign of turning panics on malformed input into proper error messages.

This release also folds in the 0.9.2 changes: 0.9.2 was prepared in-tree but no release was cut from it, so everything below covers the full 0.9.1 → 0.10.0 range.

Five people made their first contribution to findutils in this cycle: @SAY-5, @lhecker, @l46983284-cpu, @wtcpython and @aki-kong.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

## GNU Test Suite Compatibility

Compatibility is tracked per individual test (PASS/FAIL/SKIP by test name). 0.10.0 stands at:

| Result | find (GNU suite) | bfs suite |
|--------|------------------|-----------|
| Pass   | 417              | 267       |
| Fail   | 77               | 41        |
| Skip   | 1                | 6         |
| Total  | 495              | 314       |

Compared to 0.9.1, one more GNU test passes (417 vs 416). The bfs suite gained a test upstream (314 vs 313) which findutils does not pass yet, so the pass count is unchanged.

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## Highlights

### GNU compatibility fixes
* `find -printf`: print a fixed-width fraction for `%T+` — chrono's `%.f` drops the fraction (and the leading dot) when the sub-second part is zero, while GNU always prints a dot followed by ten fractional digits, by @sylvestre in https://github.com/uutils/findutils/pull/792
* `find`: reject invalid `-newerXY` predicates like GNU — the parser used an unanchored regex, so tokens such as `-neweraBcmty` were accepted as a truncated `-neweraB`, by @l46983284-cpu in https://github.com/uutils/findutils/pull/784
* `find`: reject invalid UTF-8 in `--files0-from` by @wtcpython in https://github.com/uutils/findutils/pull/798
* `xargs`: cap the default command line size at 128 KiB — sizing command lines up to `ARG_MAX` made `execvp()` fail with `E2BIG`, since the kernel also charges the argv/envp pointer arrays against the limit. An explicit `-s` may still raise the limit, as with GNU. By @sylvestre in https://github.com/uutils/findutils/pull/791
* `xargs`: account for quoting on Windows when computing length limits by @lhecker in https://github.com/uutils/findutils/pull/788

### Robustness — errors instead of panics
* `find`: report an unparsable `-newerXt` year instead of panicking by @leeewee in https://github.com/uutils/findutils/pull/762
* `locate`: report write errors instead of panicking by @SAY-5 in https://github.com/uutils/findutils/pull/785
* `locate`: ignore an unrepresentable `--max-database-age` instead of panicking by @leeewee in https://github.com/uutils/findutils/pull/768
* `find`: skip broken pipe errors in wasm executions by @aki-kong in https://github.com/uutils/findutils/pull/802

### Error messages
* `find`: avoid the duplicated `find: find:` prefix in two error messages by @cakebaker in https://github.com/uutils/findutils/pull/774

### Project & CI
* clippy: fix warnings (`byte_char_slices`, `useless_borrows_in_formatting`, `question_mark`, `manual_assert_eq`) by @cakebaker in https://github.com/uutils/findutils/pull/755
* Release 0.9.2 by @sylvestre in https://github.com/uutils/findutils/pull/793
* Release 0.10.0 by @sylvestre in https://github.com/uutils/findutils/pull/804

### Dependencies
Dependency and GitHub Action bumps via Dependabot and manual refreshes by @cakebaker: `anyhow` (1.0.102 → 1.0.103, #751), `crossbeam-epoch` (0.9.18 → 0.9.20, #754), `clap` (4.6.1 → 4.6.4, #763/#787/#790), `regex` (1.12.4 → 1.13.1, #765), `ctor` (1.0.7 → 1.0.12, #752/#764/#789/#796/#800), `thiserror` (2.0.18 → 2.0.19, #786), and `CodSpeedHQ/action` (4 → 5.0.1, #799/#803).

## New Contributors
* @SAY-5 made their first contribution in https://github.com/uutils/findutils/pull/785
* @lhecker made their first contribution in https://github.com/uutils/findutils/pull/788
* @l46983284-cpu made their first contribution in https://github.com/uutils/findutils/pull/784
* @wtcpython made their first contribution in https://github.com/uutils/findutils/pull/798
* @aki-kong made their first contribution in https://github.com/uutils/findutils/pull/802

**Full Changelog**: https://github.com/uutils/findutils/compare/0.9.1...0.10.0


## Install findutils 0.10.0

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/uutils/findutils/releases/download/0.10.0/findutils-installer.sh | sh
```

## Download findutils 0.10.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux (glibc) | [checksum](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |
| [findutils-x86_64-unknown-linux-musl.tar.xz](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-unknown-linux-musl.tar.xz) | x64 Linux (static musl) | [checksum](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-x86_64-unknown-linux-musl.tar.xz.sha256) |
| [findutils-aarch64-unknown-linux-musl.tar.xz](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-aarch64-unknown-linux-musl.tar.xz) | ARM64 Linux (static musl) | [checksum](https://github.com/uutils/findutils/releases/download/0.10.0/findutils-aarch64-unknown-linux-musl.tar.xz.sha256) |
