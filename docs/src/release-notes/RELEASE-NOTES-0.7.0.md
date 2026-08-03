# 📦 Findutils 0.7.0 Release:

We are thrilled to announce the release of Findutils 0.7.0! This version brings numerous enhancements, bug fixes, and new features, further improving the toolset's robustness and functionality. In this release, we have 86 new BFS and GNU tests passing!
Congratulations to @hanbings for pushing so many changes as part of the Google summer of code 2024.

This release saw contributions from 5 developers, including 1 newcomer.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

## BFS Test Suite Compatibility
Here’s how version 0.7.0 compares to the previous release:

| Result | Previous | Current | Change | % Total Previous | % Total Current | % Change |
|--------|----------|---------|--------|------------------|-----------------|----------|
| Pass   | 190      | 244     | +54    | 65.97%           | 76.97%          | +11%  |
| Skip   | 1        | 6       | +5     | 0.35%            | 1.89%           | +1.54%   |
| Fail   | 97       | 67      | -30    | 33.68%           | 21.14%          | -12.54%  |
| Total  | 288      | 317     | +29    |                  |                 |          |

<img src="https://github.com/uutils/findutils-tracking/blob/main/bfs-results.png?raw=true" alt="BFS testsuite evolution" width="600"/>

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## GNU Test Suite Compatibility
Here’s how version 0.7.0 compares to the previous release:

| Result | Previous | Current | Change | % Total Previous | % Total Current | % Change |
|--------|----------|---------|--------|------------------|-----------------|----------|
| Pass   | 445      | 477     | +32    | 63.38%           | 72.38%          | +9%   |
| Skip   | 1        | 1       | 0      | 0.14%            | 0.15%           | +0.01%   |
| Fail   | 254      | 179     | -75    | 36.18%           | 27.16%          | -9.02%   |
| Error  | 2        | 2       | 0      | 0.28%            | 0.30%           | +0.02%   |
| Total  | 702      | 659     | -43    |                  |                 |          |

<img src="https://github.com/uutils/findutils-tracking/blob/main/gnu-results.png?raw=true" alt="GNU testsuite evolution" width="600"/>

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## Download findutils 0.7.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.7.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |

## What's Changed

### Find
* Implement `-noleaf` by @hanbings in https://github.com/uutils/findutils/pull/414
* Implement simple caching for -fstype by @nerdroychan in https://github.com/uutils/findutils/pull/419
* Implement `-daystart` by @hanbings in https://github.com/uutils/findutils/pull/413
 * find: ctime is "changed" time, not "created" time by @tavianator in https://github.com/uutils/findutils/pull/416
* find: -daystart returns true by @tavianator in https://github.com/uutils/findutils/pull/428
* Implement `-fprint` by @hanbings in https://github.com/uutils/findutils/pull/421
* Wrap walkdir::DirEntry in a new type by @tavianator in https://github.com/uutils/findutils/pull/436
* Implement `-ls` and `-fls` by @hanbings in https://github.com/uutils/findutils/pull/435

### CI
* ci: Update bfs to 4.0 by @tavianator in https://github.com/uutils/findutils/pull/446

### Code quality
* options return true by @tavianator in https://github.com/uutils/findutils/pull/417
* find: fix needless borrow clippy warning by @cakebaker in https://github.com/uutils/findutils/pull/423
* Remove unnecessary lifetime parameter from Dependencies by @tavianator in https://github.com/uutils/findutils/pull/427
* find: use array instead of closure by @cakebaker in https://github.com/uutils/findutils/pull/450
* Clean up unused generated during Windows platform build process. by @hanbings in https://github.com/uutils/findutils/pull/448

### Dependencies
* build(deps): bump clap from 4.5.7 to 4.5.8 by @dependabot in https://github.com/uutils/findutils/pull/410
* build(deps): bump clap from 4.5.8 to 4.5.9 by @dependabot in https://github.com/uutils/findutils/pull/415
* build(deps): bump clap from 4.5.9 to 4.5.10 by @dependabot in https://github.com/uutils/findutils/pull/422
* build(deps): bump clap from 4.5.10 to 4.5.13 by @dependabot in https://github.com/uutils/findutils/pull/429
* build(deps): bump regex from 1.10.5 to 1.10.6 by @dependabot in https://github.com/uutils/findutils/pull/432
* build(deps): bump tempfile from 3.10.1 to 3.11.0 by @dependabot in https://github.com/uutils/findutils/pull/433
* build(deps): bump assert_cmd from 2.0.14 to 2.0.15 by @dependabot in https://github.com/uutils/findutils/pull/426
* build(deps): bump predicates from 3.1.0 to 3.1.2 by @dependabot in https://github.com/uutils/findutils/pull/424
* build(deps): bump tempfile from 3.11.0 to 3.12.0 by @dependabot in https://github.com/uutils/findutils/pull/434
* build(deps): bump clap from 4.5.13 to 4.5.14 by @dependabot in https://github.com/uutils/findutils/pull/438
* build(deps): bump assert_cmd from 2.0.15 to 2.0.16 by @dependabot in https://github.com/uutils/findutils/pull/437
* build(deps): bump clap from 4.5.14 to 4.5.15 by @dependabot in https://github.com/uutils/findutils/pull/441
* build(deps): bump filetime from 0.2.23 to 0.2.24 by @dependabot in https://github.com/uutils/findutils/pull/442
* build(deps): bump clap from 4.5.15 to 4.5.16 by @dependabot in https://github.com/uutils/findutils/pull/445
* build(deps): bump filetime from 0.2.24 to 0.2.25 by @dependabot in https://github.com/uutils/findutils/pull/447
* build(deps): bump clap from 4.5.16 to 4.5.17 by @dependabot in https://github.com/uutils/findutils/pull/449

## New Contributors
* @nerdroychan made their first contribution in https://github.com/uutils/findutils/pull/419

**Full Changelog**: https://github.com/uutils/findutils/compare/0.6.0...0.7.0
