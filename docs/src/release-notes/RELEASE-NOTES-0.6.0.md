# 📦 Findutils 0.6.0 Release:

We are thrilled to announce the release of Findutils 0.6.0! This version brings numerous enhancements, bug fixes, and new features, further improving the toolset's robustness and functionality.

This release saw contributions from 8 developers, including 5 newcomers. Including @hanbings as part of the Google summer of code 2024.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

## BFS Test Suite Compatibility
Here’s how version 0.6.0 compares to the previous release:

| Result        | Previous | Current | Change       | % Total Previous | % Total Current | % Change          |
|---------------|----------|---------|--------------|------------------|-----------------|-------------------|
| Pass          | 176      | 190     | +14          | 61.11%           | 65.97%          | +4.86%            |
| Skip          | 1        | 1       | 0            | 0.35%            | 0.35%           | 0.00%             |
| Fail          | 111      | 97      | -14          | 38.54%           | 33.68%          | -4.86%            |

<img src="https://github.com/uutils/findutils-tracking/blob/main/bfs-results.png?raw=true" alt="BFS testsuite evolution" width="600"/>

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## GNU Test Suite Compatibility
Here’s how version 0.6.0 compares to the previous release:

| Result        | Previous | Current | Change       | % Total Previous | % Total Current | % Change          |
|---------------|----------|---------|--------------|------------------|-----------------|-------------------|
| Pass          | 429      | 445     | +16          | 59.75%           | 63.38%          | +3.63%            |
| Skip          | 1        | 1       | 0            | 0.14%            | 0.14%           | 0.00%             |
| Fail          | 286      | 254     | -32          | 39.83%           | 36.18%          | -3.65%            |
| Error         | 2        | 2       | 0            | 0.28%            | 0.28%           | 0.00%             |

<img src="https://github.com/uutils/findutils-tracking/blob/main/gnu-results.png?raw=true" alt="GNU testsuite evolution" width="600"/>

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## Download findutils 0.6.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.6.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |

## What's Changed

### Find

* Implement `-uid` and `-gid` by @hanbings in https://github.com/uutils/findutils/pull/405
* Implement `-samefile` by @hanbings in https://github.com/uutils/findutils/pull/389
* Implement `-fstype` by @hanbings in https://github.com/uutils/findutils/pull/408
* Cache FileInformation for the target by @tavianator in https://github.com/uutils/findutils/pull/409
* remove unused import on Windows by @cakebaker in https://github.com/uutils/findutils/pull/395
* Fix `-newerXY` test code for Windows platform. by @hanbings in https://github.com/uutils/findutils/pull/394
* Implement `-amin` `-cmin` `-mmin` age ranges. by @hanbings in https://github.com/uutils/findutils/pull/355
* Implement `-[no]user` and `-[no]group` predicates. by @hanbings in https://github.com/uutils/findutils/pull/368
* Implement `-newerXY` by @hanbings in https://github.com/uutils/findutils/pull/342
* Implement `-anewer` and `-cnewer`. by @hanbings in https://github.com/uutils/findutils/pull/386
* fix "item x imported redundantly" warnings by @cakebaker in https://github.com/uutils/findutils/pull/326
* make `InodeMatcher` & `LinksMatcher` unix-only by @cakebaker in https://github.com/uutils/findutils/pull/393
* fix usage of legacy numeric methods by @cakebaker in https://github.com/uutils/findutils/pull/403
* Fix panic when piped into head by @hanbings in https://github.com/uutils/findutils/pull/351
* Changed exit code when an error is encountered in `process_dir()`. by @hanbings in https://github.com/uutils/findutils/pull/352
* Fix unexpectedly accepted empty expression. by @hanbings in https://github.com/uutils/findutils/pull/353
* Fix `convert_arg_to_comparable_value()` function parsing unexpected characters. by @hanbings in https://github.com/uutils/findutils/pull/361
* replace `zip` with nested loops in tests by @cakebaker in https://github.com/uutils/findutils/pull/385

### xargs
* remove unnecessary else blocks by @cakebaker in https://github.com/uutils/findutils/pull/318
* rename `--size` to `--max-chars` by @cakebaker in https://github.com/uutils/findutils/pull/321
* do not merge extra args before replacing by @YDX-2147483647 in https://github.com/uutils/findutils/pull/363
* add --max-lines by @cakebaker in https://github.com/uutils/findutils/pull/343
* Implement replace / -I (Closes: #310) by @sylvestre in https://github.com/uutils/findutils/pull/323

### Build

* Cargo.toml: set "default-features = false" for onig by @cakebaker in https://github.com/uutils/findutils/pull/313
* Cargo.toml: use 2021 edition by @cakebaker in https://github.com/uutils/findutils/pull/314

### Code quality
* Fix clippy warning on 'format!' strings by @jellehelsen in https://github.com/uutils/findutils/pull/272
* Fix deprecation warnings from `chrono` in test by @cakebaker in https://github.com/uutils/findutils/pull/315
* tests: remove "extern crate x" by @cakebaker in https://github.com/uutils/findutils/pull/341
* xargs: use `#[allow(dead_code)]` for enum by @cakebaker in https://github.com/uutils/findutils/pull/396
* Run clippy pedantic fixes on the recent changes by @sylvestre in https://github.com/uutils/findutils/pull/349
* Run clippy pedantic fixes by @sylvestre in https://github.com/uutils/findutils/pull/348
* find: fix two clippy warnings by @cakebaker in https://github.com/uutils/findutils/pull/335
* clippy: Fix clippy warning. by @hanbings in https://github.com/uutils/findutils/pull/357

### Documentation
* initial oranda setup by @tertsdiepraam in https://github.com/uutils/findutils/pull/274
* website: fix `path_prefix` in oranda by @tertsdiepraam in https://github.com/uutils/findutils/pull/275
* website: fix changelog config by @tertsdiepraam in https://github.com/uutils/findutils/pull/278
* README: add badges + bfs info by @sylvestre in https://github.com/uutils/findutils/pull/337
* docs: initial version of mdbook by @tertsdiepraam in https://github.com/uutils/findutils/pull/347

### CI
* Provide GNU test comparison comments for PRs in Github Actions. by @hanbings in https://github.com/uutils/findutils/pull/400
* ci: Update bfs to 3.0.3 by @tavianator in https://github.com/uutils/findutils/pull/290
* ci: Update bfs to 3.1.3 by @tavianator in https://github.com/uutils/findutils/pull/334
* Fix CI code for `comment.yml` by @hanbings in https://github.com/uutils/findutils/pull/404
* Comments are only sent when GNU/BFS test compatibility changes. by @hanbings in https://github.com/uutils/findutils/pull/407
* ci: run `cargo test` instead of `cargo check` in "cargo test" jobs by @cakebaker in https://github.com/uutils/findutils/pull/392
* ci: name upload steps by @cakebaker in https://github.com/uutils/findutils/pull/346
* ci: use `--all-targets` when running clippy by @cakebaker in https://github.com/uutils/findutils/pull/359
* use cargo-dist + prepare 0.5.0 by @sylvestre in https://github.com/uutils/findutils/pull/350
* cargo-dist: generate more targets by @sylvestre in https://github.com/uutils/findutils/pull/388
* ci: update codecov-action to v4 & use token by @cakebaker in https://github.com/uutils/findutils/pull/320

### Dependencies

* Cargo.toml: specify complete chrono version by @cakebaker in https://github.com/uutils/findutils/pull/339
* Bump wasm-bindgen from 0.2.82 to 0.2.91 by @cakebaker in https://github.com/uutils/findutils/pull/317
* Bump parking_lot_core from 0.9.3 to 0.9.9 by @cakebaker in https://github.com/uutils/findutils/pull/316
* Bump clap from 4.4.6 to 4.4.18 by @cakebaker in https://github.com/uutils/findutils/pull/319
* Bump nix from 0.27 to 0.28 & enable "fs" feature by @cakebaker in https://github.com/uutils/findutils/pull/327
* build(deps): bump tempfile from 3.6.0 to 3.7.0 by @dependabot in https://github.com/uutils/findutils/pull/269
* build(deps): bump assert_cmd from 2.0.11 to 2.0.12 by @dependabot in https://github.com/uutils/findutils/pull/268
* build(deps): bump actions/checkout from 3 to 4 by @dependabot in https://github.com/uutils/findutils/pull/283
* build(deps): bump uucore from 0.0.20 to 0.0.21 by @dependabot in https://github.com/uutils/findutils/pull/282
* build(deps): bump tempfile from 3.7.0 to 3.8.0 by @dependabot in https://github.com/uutils/findutils/pull/276
* build(deps): bump filetime from 0.2.21 to 0.2.22 by @dependabot in https://github.com/uutils/findutils/pull/271
* build(deps): bump chrono from 0.4.26 to 0.4.28 by @dependabot in https://github.com/uutils/findutils/pull/281
* build(deps): bump walkdir from 2.3.3 to 2.4.0 by @dependabot in https://github.com/uutils/findutils/pull/284
* build(deps): bump chrono from 0.4.28 to 0.4.29 by @dependabot in https://github.com/uutils/findutils/pull/285
* build(deps): bump chrono from 0.4.29 to 0.4.31 by @dependabot in https://github.com/uutils/findutils/pull/288
* build(deps): bump uucore from 0.0.21 to 0.0.22 by @dependabot in https://github.com/uutils/findutils/pull/291
* build(deps): bump predicates from 3.0.3 to 3.0.4 by @dependabot in https://github.com/uutils/findutils/pull/289
* build(deps): bump nix from 0.26.2 to 0.27.1 by @dependabot in https://github.com/uutils/findutils/pull/279
* build(deps): bump dawidd6/action-download-artifact from 2 to 3 by @dependabot in https://github.com/uutils/findutils/pull/297
* build(deps): bump once_cell from 1.18.0 to 1.19.0 by @dependabot in https://github.com/uutils/findutils/pull/296
* build(deps): bump filetime from 0.2.22 to 0.2.23 by @dependabot in https://github.com/uutils/findutils/pull/295
* build(deps): bump tempfile from 3.8.0 to 3.8.1 by @dependabot in https://github.com/uutils/findutils/pull/293
* build(deps): bump uucore from 0.0.22 to 0.0.23 by @dependabot in https://github.com/uutils/findutils/pull/294
* build(deps): bump actions/upload-artifact from 3 to 4 by @dependabot in https://github.com/uutils/findutils/pull/298
* build(deps): bump tempfile from 3.8.1 to 3.9.0 by @dependabot in https://github.com/uutils/findutils/pull/299
* build(deps): bump assert_cmd from 2.0.12 to 2.0.13 by @dependabot in https://github.com/uutils/findutils/pull/301
* build(deps): bump predicates from 3.0.4 to 3.1.0 by @dependabot in https://github.com/uutils/findutils/pull/302
* build(deps): bump chrono from 0.4.31 to 0.4.32 by @dependabot in https://github.com/uutils/findutils/pull/304
* build(deps): bump shlex from 1.1.0 to 1.3.0 by @dependabot in https://github.com/uutils/findutils/pull/303
* build(deps): bump uucore from 0.0.23 to 0.0.24 by @dependabot in https://github.com/uutils/findutils/pull/305
* build(deps): bump chrono from 0.4.32 to 0.4.33 by @dependabot in https://github.com/uutils/findutils/pull/306
* build(deps): bump chrono from 0.4.33 to 0.4.34 by @dependabot in https://github.com/uutils/findutils/pull/311
* build(deps): bump tempfile from 3.9.0 to 3.10.0 by @dependabot in https://github.com/uutils/findutils/pull/309
* build(deps): bump serial_test from 2.0.0 to 3.0.0 by @dependabot in https://github.com/uutils/findutils/pull/300
* build(deps): bump assert_cmd from 2.0.13 to 2.0.14 by @dependabot in https://github.com/uutils/findutils/pull/312
* build(deps): bump tempfile from 3.10.0 to 3.10.1 by @dependabot in https://github.com/uutils/findutils/pull/328
* build(deps): bump walkdir from 2.4.0 to 2.5.0 by @dependabot in https://github.com/uutils/findutils/pull/329
* build(deps): bump chrono from 0.4.34 to 0.4.35 by @dependabot in https://github.com/uutils/findutils/pull/330
* build(deps): bump clap from 4.4.18 to 4.5.2 by @dependabot in https://github.com/uutils/findutils/pull/332
* build(deps): bump clap from 4.5.2 to 4.5.3 by @dependabot in https://github.com/uutils/findutils/pull/333
* build(deps): bump uucore from 0.0.24 to 0.0.25 by @dependabot in https://github.com/uutils/findutils/pull/336
* build(deps): bump clap from 4.5.3 to 4.5.4 by @dependabot in https://github.com/uutils/findutils/pull/340
* build(deps): bump chrono from 0.4.35 to 0.4.37 by @dependabot in https://github.com/uutils/findutils/pull/344
* build(deps): bump KyleMayes/install-llvm-action from 1 to 2 by @dependabot in https://github.com/uutils/findutils/pull/345
* build(deps): bump chrono from 0.4.37 to 0.4.38 by @dependabot in https://github.com/uutils/findutils/pull/354
* build(deps): bump serial_test from 3.0.0 to 3.1.0 by @dependabot in https://github.com/uutils/findutils/pull/356
* build(deps): bump uucore from 0.0.25 to 0.0.26 by @dependabot in https://github.com/uutils/findutils/pull/365
* build(deps): bump serial_test from 3.1.0 to 3.1.1 by @dependabot in https://github.com/uutils/findutils/pull/364
* build(deps): bump nix from 0.28.0 to 0.29.0 by @dependabot in https://github.com/uutils/findutils/pull/390
* build(deps): bump dawidd6/action-download-artifact from 3 to 4 by @dependabot in https://github.com/uutils/findutils/pull/397
* build(deps): bump dawidd6/action-download-artifact from 4 to 5 by @dependabot in https://github.com/uutils/findutils/pull/398
* build(deps): bump clap from 4.5.4 to 4.5.6 by @dependabot in https://github.com/uutils/findutils/pull/399
* build(deps): bump clap from 4.5.6 to 4.5.7 by @dependabot in https://github.com/uutils/findutils/pull/401
* build(deps): bump dawidd6/action-download-artifact from 5 to 6 by @dependabot in https://github.com/uutils/findutils/pull/402
* build(deps): bump uucore from 0.0.26 to 0.0.27 by @dependabot in https://github.com/uutils/findutils/pull/406
* Bump regex from 1.7 to 1.10 by @cakebaker in https://github.com/uutils/findutils/pull/391
* xargs: upgrade to clap v4.4 by @cakebaker in https://github.com/uutils/findutils/pull/331

## New Contributors
* @tertsdiepraam made their first contribution in https://github.com/uutils/findutils/pull/274
* @jellehelsen made their first contribution in https://github.com/uutils/findutils/pull/272
* @cakebaker made their first contribution in https://github.com/uutils/findutils/pull/313
* @hanbings made their first contribution in https://github.com/uutils/findutils/pull/342
* @YDX-2147483647 made their first contribution in https://github.com/uutils/findutils/pull/363

**Full Changelog**: https://github.com/uutils/findutils/compare/0.4.2...0.6.0
