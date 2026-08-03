# 📦 Findutils 0.8.0 Release:

We are thrilled to announce the release of Findutils 0.8.0! This version brings numerous enhancements, bug fixes, and new features, further improving the toolset's robustness and functionality. This release introduces key improvements including new formatting options (`-fprint0`, `-fprintf`), better file handling with `-files0-from`, and more efficient command execution with `exec[dir] +`. We've also fixed important path handling bugs and improved regex behavior.

This release saw contributions from 5 new developers.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

## GNU Test Suite Compatibility
Here's how version 0.8.0 compares to the previous release:

| Result | Previous (0.7.0) | Current (0.8.0) | Change | % Total Previous | % Total Current | % Change |
|--------|----------|---------|--------|------------------|-----------------|----------|
| Pass   | 477      | 503     | +26    | 72.38%           | 80.87%          | +8.49%  |
| Skip   | 1        | 1       | 0      | 0.15%            | 0.16%           | +0.01%  |
| Fail   | 179      | 117     | -62    | 27.16%           | 18.81%          | -8.35%  |
| Error  | 2        | 1       | -1     | 0.30%            | 0.16%           | -0.14%  |
| Total  | 659      | 622     | -37    |                  |                 |          |

<img src="https://github.com/uutils/findutils-tracking/blob/main/gnu-results.png?raw=true" alt="GNU testsuite evolution" width="600"/>

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## What's Changed
* Implement `-fprint0` by @hanbings in https://github.com/uutils/findutils/pull/443
* Implement `-fprintf` by @hanbings in https://github.com/uutils/findutils/pull/444
* manage the special case 'find %A+ by @sylvestre in https://github.com/uutils/findutils/pull/454
* Print help/version without error prefix by @andrewliebenow in https://github.com/uutils/findutils/pull/455
* find/glob: Don't use `$.` as a non-matching regex by @tavianator in https://github.com/uutils/findutils/pull/457
* find/time: calculate midnight with local time zone by @NaviHX in https://github.com/uutils/findutils/pull/467
* find: fix new clippy warnings by @cakebaker in https://github.com/uutils/findutils/pull/475
* ci: Fix issues related to downloading builds in ci. by @hanbings in https://github.com/uutils/findutils/pull/487
* ci: use `-Cinstrument-coverage` instead of `-Zprofile` by @cakebaker in https://github.com/uutils/findutils/pull/470
* find: fix "unused import" warning on Windows by @cakebaker in https://github.com/uutils/findutils/pull/498
* -name / should match /// by @Every2 in https://github.com/uutils/findutils/pull/508
* Fix incorrect triple slashes behavior by @Every2 in https://github.com/uutils/findutils/pull/512
* find: Use matcher_io.set_exit_code() by @tavianator in https://github.com/uutils/findutils/pull/516
* clippy: add the same rules as coreutils by @sylvestre in https://github.com/uutils/findutils/pull/510
* clippy: run cargo clippy and cargo fmt. by @hanbings in https://github.com/uutils/findutils/pull/517
* Implement : -files0-from (#378) by @Crypto-Darth in https://github.com/uutils/findutils/pull/509
* find: Implement exec[dir] + by @milkory in https://github.com/uutils/findutils/pull/515
* tests: Make -exec {} + tests deterministic by @tavianator in https://github.com/uutils/findutils/pull/520
* clippy: fix warnings from Rust 1.86 by @cakebaker in https://github.com/uutils/findutils/pull/522
* find: User/group ID fixes by @tavianator in https://github.com/uutils/findutils/pull/521
* ci: adjust to code cov v5 by @sylvestre in https://github.com/uutils/findutils/pull/525
* find: fix typo in test (slashs -> slashes) by @cakebaker in https://github.com/uutils/findutils/pull/526
* prepare version 0.8.0 by @sylvestre in https://github.com/uutils/findutils/pull/528
* find: Allow comparable values with signs by @tavianator in https://github.com/uutils/findutils/pull/524

### Dependencies
* build(deps): bump pretty_assertions from 1.4.0 to 1.4.1 by @dependabot in https://github.com/uutils/findutils/pull/453
* build(deps): bump once_cell from 1.19.0 to 1.20.0 by @dependabot in https://github.com/uutils/findutils/pull/452
* build(deps): bump clap from 4.5.17 to 4.5.18 by @dependabot in https://github.com/uutils/findutils/pull/456
* build(deps): bump regex from 1.10.6 to 1.11.0 by @dependabot in https://github.com/uutils/findutils/pull/460
* build(deps): bump tempfile from 3.12.0 to 3.13.0 by @dependabot in https://github.com/uutils/findutils/pull/458
* build(deps): bump once_cell from 1.20.0 to 1.20.2 by @dependabot in https://github.com/uutils/findutils/pull/464
* build(deps): bump clap from 4.5.18 to 4.5.20 by @dependabot in https://github.com/uutils/findutils/pull/465
* build(deps): bump regex from 1.11.0 to 1.11.1 by @dependabot in https://github.com/uutils/findutils/pull/468
* build(deps): bump serial_test from 3.1.1 to 3.2.0 by @dependabot in https://github.com/uutils/findutils/pull/471
* build(deps): bump clap from 4.5.20 to 4.5.31 by @dependabot in https://github.com/uutils/findutils/pull/496
* build(deps): bump once_cell from 1.20.2 to 1.20.3 by @dependabot in https://github.com/uutils/findutils/pull/491
* build(deps): bump predicates from 3.1.2 to 3.1.3 by @dependabot in https://github.com/uutils/findutils/pull/485
* build(deps): bump tempfile from 3.13.0 to 3.17.1 by @dependabot in https://github.com/uutils/findutils/pull/495
* build(deps): bump once_cell from 1.20.3 to 1.21.0 by @dependabot in https://github.com/uutils/findutils/pull/502
* build(deps): bump tempfile from 3.18.0 to 3.19.0 by @dependabot in https://github.com/uutils/findutils/pull/505
* build(deps): bump clap from 4.5.32 to 4.5.34 by @dependabot in https://github.com/uutils/findutils/pull/513
* Update to uucore 0.0.29 by @sylvestre in https://github.com/uutils/findutils/pull/486
* Bump `uucore` from `0.0.29` to `0.0.30` by @cakebaker in https://github.com/uutils/findutils/pull/514
* build(deps): bump tempfile from 3.17.1 to 3.18.0 by @dependabot in https://github.com/uutils/findutils/pull/500
* build(deps): bump clap from 4.5.31 to 4.5.32 by @dependabot in https://github.com/uutils/findutils/pull/501
* build(deps): bump tempfile from 3.19.0 to 3.19.1 by @dependabot in https://github.com/uutils/findutils/pull/507
* Bump dependencies to remove two `windows-sys` versions by @cakebaker in https://github.com/uutils/findutils/pull/499
* Remove unused `once_cell` dependency by @cakebaker in https://github.com/uutils/findutils/pull/503
* build(deps): bump chrono from 0.4.38 to 0.4.40 by @dependabot in https://github.com/uutils/findutils/pull/497
* build(deps): bump clap from 4.5.34 to 4.5.35 by @dependabot in https://github.com/uutils/findutils/pull/519
* ci: update `dist` from `0.12` to `0.28` by @cakebaker in https://github.com/uutils/findutils/pull/506

## New Contributors
* @andrewliebenow made their first contribution in https://github.com/uutils/findutils/pull/455
* @NaviHX made their first contribution in https://github.com/uutils/findutils/pull/467
* @Every2 made their first contribution in https://github.com/uutils/findutils/pull/508
* @Crypto-Darth made their first contribution in https://github.com/uutils/findutils/pull/509
* @milkory made their first contribution in https://github.com/uutils/findutils/pull/515

**Full Changelog**: https://github.com/uutils/findutils/compare/0.7.0...0.8.0

## Install findutils 0.8.0

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/uutils/findutils/releases/download/0.8.0/findutils-installer.sh | sh
```

## Download findutils 0.8.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.8.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |
