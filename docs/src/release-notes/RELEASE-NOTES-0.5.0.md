# 📦 Findutils 0.5.0 Release

## Download findutils 0.5.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.5.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |

## What's Changed

### New features
* find: Implement `-newerXY` by @hanbings in https://github.com/uutils/findutils/pull/342
* xargs: Implement replace / -I (Closes: #310) by @sylvestre in https://github.com/uutils/findutils/pull/323
* xargs: add --max-lines by @cakebaker in https://github.com/uutils/findutils/pull/343
* xargs: rename `--size` to `--max-chars` by @cakebaker in https://github.com/uutils/findutils/pull/321

### Documentation
* initial oranda setup by @tertsdiepraam in https://github.com/uutils/findutils/pull/274
* website: fix `path_prefix` in oranda by @tertsdiepraam in https://github.com/uutils/findutils/pull/275
* website: fix changelog config by @tertsdiepraam in https://github.com/uutils/findutils/pull/278
* docs: initial version of mdbook by @tertsdiepraam in https://github.com/uutils/findutils/pull/347

### Code quality
* Fix deprecation warnings from `chrono` in test by @cakebaker in https://github.com/uutils/findutils/pull/315
* xargs: remove unnecessary else blocks by @cakebaker in https://github.com/uutils/findutils/pull/318
* find: fix "item x imported redundantly" warnings by @cakebaker in https://github.com/uutils/findutils/pull/326
* find: fix two clippy warnings by @cakebaker in https://github.com/uutils/findutils/pull/335
* README: add badges + bfs info by @sylvestre in https://github.com/uutils/findutils/pull/337
* tests: remove "extern crate x" by @cakebaker in https://github.com/uutils/findutils/pull/341
* Run clippy pedantic fixes by @sylvestre in https://github.com/uutils/findutils/pull/348

### CI
* ci: update codecov-action to v4 & use token by @cakebaker in https://github.com/uutils/findutils/pull/320
* Fix clippy warning on 'format!' strings by @jellehelsen in https://github.com/uutils/findutils/pull/272
* ci: Update bfs to 3.0.3 by @tavianator in https://github.com/uutils/findutils/pull/290
* ci: Update bfs to 3.1.3 by @tavianator in https://github.com/uutils/findutils/pull/334
* Cargo.toml: set "default-features = false" for onig by @cakebaker in https://github.com/uutils/findutils/pull/313
* Cargo.toml: use 2021 edition by @cakebaker in https://github.com/uutils/findutils/pull/314
* ci: name upload steps by @cakebaker in https://github.com/uutils/findutils/pull/346

### dependencies
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
* Bump wasm-bindgen from 0.2.82 to 0.2.91 by @cakebaker in https://github.com/uutils/findutils/pull/317
* Bump clap from 4.4.6 to 4.4.18 by @cakebaker in https://github.com/uutils/findutils/pull/319
* build(deps): bump tempfile from 3.10.0 to 3.10.1 by @dependabot in https://github.com/uutils/findutils/pull/328
* build(deps): bump walkdir from 2.4.0 to 2.5.0 by @dependabot in https://github.com/uutils/findutils/pull/329
* build(deps): bump chrono from 0.4.34 to 0.4.35 by @dependabot in https://github.com/uutils/findutils/pull/330
* xargs: upgrade to clap v4.4 by @cakebaker in https://github.com/uutils/findutils/pull/331
* build(deps): bump clap from 4.4.18 to 4.5.2 by @dependabot in https://github.com/uutils/findutils/pull/332
* build(deps): bump clap from 4.5.2 to 4.5.3 by @dependabot in https://github.com/uutils/findutils/pull/333
* build(deps): bump clap from 4.5.3 to 4.5.4 by @dependabot in https://github.com/uutils/findutils/pull/340
* build(deps): bump KyleMayes/install-llvm-action from 1 to 2 by @dependabot in https://github.com/uutils/findutils/pull/345
* build(deps): bump chrono from 0.4.35 to 0.4.37 by @dependabot in https://github.com/uutils/findutils/pull/344
* Bump nix from 0.27 to 0.28 & enable "fs" feature by @cakebaker in https://github.com/uutils/findutils/pull/327
* Bump parking_lot_core from 0.9.3 to 0.9.9 by @cakebaker in https://github.com/uutils/findutils/pull/316
* build(deps): bump uucore from 0.0.24 to 0.0.25 by @dependabot in https://github.com/uutils/findutils/pull/336
* Cargo.toml: specify complete chrono version by @cakebaker in https://github.com/uutils/findutils/pull/339

## New Contributors
* @tertsdiepraam made their first contribution in https://github.com/uutils/findutils/pull/274
* @jellehelsen made their first contribution in https://github.com/uutils/findutils/pull/272
* @cakebaker made their first contribution in https://github.com/uutils/findutils/pull/313
* @hanbings made their first contribution in https://github.com/uutils/findutils/pull/342

**Full Changelog**: https://github.com/uutils/findutils/compare/0.4.2...0.5.0
