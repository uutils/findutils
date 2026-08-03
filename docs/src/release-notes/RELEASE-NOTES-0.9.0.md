# 📦 Findutils 0.9.0 Release:

We are thrilled to announce the release of Findutils 0.9.0! This release is a major milestone: it introduces the first implementations of **`locate` and `updatedb`**, bringing the project much closer to a complete drop-in replacement for the GNU findutils suite. Alongside these new utilities, this version adds the interactive `-ok`/`-okdir` actions, new `xargs` options (`-E`, `-l`, and hyphenated `-I`/`-E` values), chained `-type` arguments, and many correctness fixes. We've also hardened the project with a security audit workflow, a `SECURITY.md`, and CodSpeed performance benchmarks for all four utilities.

This release also lands as the broader uutils effort reaches a new level of adoption: Microsoft now ships uutils-based tooling ([microsoft/coreutils](https://github.com/microsoft/coreutils)), a strong validation of the project's drop-in, cross-platform approach.

This release saw contributions from 11 new developers.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

## GNU Test Suite Compatibility
Here's how version 0.9.0 compares to the previous release:

| Result | Previous (0.8.0) | Current (0.9.0) | Change | % Total Previous | % Total Current | % Change |
|--------|----------|---------|--------|------------------|-----------------|----------|
| Pass   | 503      | 518     | +15    | 80.87%           | 83.28%          | +2.41%  |
| Skip   | 1        | 1       | 0      | 0.16%            | 0.16%           | 0.00%   |
| Fail   | 117      | 103     | -14    | 18.81%           | 16.56%          | -2.25%  |
| Error  | 1        | 0       | -1     | 0.16%            | 0.00%           | -0.16%  |
| Total  | 622      | 622     | 0      |                  |                 |          |

<img src="https://github.com/uutils/findutils-tracking/blob/main/gnu-results.png?raw=true" alt="GNU testsuite evolution" width="600"/>

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## Highlights

### New utilities: `locate` and `updatedb`
* Initial implementation of `locate` and `updatedb` by @Qelxiros in https://github.com/uutils/findutils/pull/536
* `locate`/`updatedb` made to work on Windows by @sylvestre in https://github.com/uutils/findutils/pull/704
* `updatedb`: improve perf and error messages, and a clear error when the output database can't be created by @sylvestre in https://github.com/uutils/findutils/pull/713

### find
* Implement `-ok` and `-okdir` by @jmr in https://github.com/uutils/findutils/pull/650
* Match the GNU prompt format for `-ok`/`-okdir` by @sylvestre in https://github.com/uutils/findutils/pull/699
* Implement chained `-type` matcher arguments by @Crypto-Darth in https://github.com/uutils/findutils/pull/527
* Replace the `{}` placeholder in the `-exec` utility name by @mvanhorn in https://github.com/uutils/findutils/pull/647
* Fix `-fstype` with stacked mounts by @tavianator in https://github.com/uutils/findutils/pull/532
* Avoid a redundant `stat` per `-fstype` evaluation by @sylvestre
* Avoid a panic on `-fprintf` with missing arguments by @sylvestre in https://github.com/uutils/findutils/pull/698
* Handle write failures when printing `--help`/`--version` by @Xylphy in https://github.com/uutils/findutils/pull/656
* Update the `--help` text by @jmr in https://github.com/uutils/findutils/pull/707
* Use `div_ceil` for proper blocks computation by @KartikDua1504 in https://github.com/uutils/findutils/pull/529

### xargs
* Add support for `-E` by @brian-pane in https://github.com/uutils/findutils/pull/593
* Add support for `-l` by @brian-pane in https://github.com/uutils/findutils/pull/601
* Ignore `-E`/`-e` when a delimiter is specified by @brian-pane in https://github.com/uutils/findutils/pull/603
* Accept hyphenated `-I` and `-E` values by @kevinburke in https://github.com/uutils/findutils/pull/668

### Project, CI & quality
* Add `SECURITY.md` by @xtqqczze in https://github.com/uutils/findutils/pull/663
* Add a security audit workflow by @xtqqczze in https://github.com/uutils/findutils/pull/660
* Add CodSpeed benchmarks for `find` and `xargs` by @sylvestre in https://github.com/uutils/findutils/pull/703
* Add CodSpeed benchmarks for `updatedb` and `locate` by @sylvestre in https://github.com/uutils/findutils/pull/705
* Port the tests to uutests by @jmr in https://github.com/uutils/findutils/pull/645
* Enable the clippy `all`/`cargo`/`pedantic` lint groups, mirroring uutils/coreutils by @sylvestre in https://github.com/uutils/findutils/pull/715
* Fix warnings from the `filter_next` lint by @cakebaker in https://github.com/uutils/findutils/pull/604
* Publish the release as a draft first, and publish the binary from `main` by @sylvestre / @oech3 in https://github.com/uutils/findutils/pull/530, https://github.com/uutils/findutils/pull/618
* Add a `release-small` profile and rename the dist profile by @oech3 in https://github.com/uutils/findutils/pull/586
* Faster CI: use preinstalled Rust & `CARGO_INCREMENTAL=0` by @oech3 in https://github.com/uutils/findutils/pull/619

### Dependencies
Numerous dependency bumps via Dependabot, including `clap` (4.5.35 → 4.6.1), `regex` (1.11.1 → 1.12.3), `nix` (0.29.0 → 0.31.3), `chrono` (0.4.40 → 0.4.45), `tempfile` (3.19.1 → 3.27.0), `ctor` (0.6.3 → 1.0.7), `uucore` (0.0.30 → 0.9.0), `uutests` (0.7.0 → 0.9.0), `onig` (6.4.0 → 6.5.3), `argmax` (0.3.1 → 0.4.0), `filetime`, `serial_test`, `assert_cmd`, `rstest`, `terminal_size`, and several GitHub Actions. Yanked `futures-util`, `scc`, and `getrandom 0.3.1` were removed.

## New Contributors
* @KartikDua1504 made their first contribution in https://github.com/uutils/findutils/pull/529
* @tdulcet made their first contribution in https://github.com/uutils/findutils/pull/545
* @Qelxiros made their first contribution in https://github.com/uutils/findutils/pull/547
* @oech3 made their first contribution in https://github.com/uutils/findutils/pull/586
* @brian-pane made their first contribution in https://github.com/uutils/findutils/pull/593
* @jmr made their first contribution in https://github.com/uutils/findutils/pull/645
* @xtqqczze made their first contribution in https://github.com/uutils/findutils/pull/659
* @kevinburke made their first contribution in https://github.com/uutils/findutils/pull/668
* @vip892766gma made their first contribution in https://github.com/uutils/findutils/pull/687
* @mvanhorn made their first contribution in https://github.com/uutils/findutils/pull/647
* @Xylphy made their first contribution in https://github.com/uutils/findutils/pull/656

**Full Changelog**: https://github.com/uutils/findutils/compare/0.8.0...0.9.0

## Install findutils 0.9.0

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/uutils/findutils/releases/download/0.9.0/findutils-installer.sh | sh
```

## Download findutils 0.9.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.9.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |
