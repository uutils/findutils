# 📦 Findutils 0.11.0 Release

Findutils 0.11.0 is the largest GNU-compatibility release in a while: 20 more GNU tests pass (437, up from 417) and 13 more bfs tests (280, up from 267). Most of the work went into `find -printf` (precision and flags, `\c`, `%m`, octal escapes, unknown directives) and `find -ls` (file type, special bits, block counts, symlink targets, local time), `-newerXt` date parsing, and `xargs` input splitting (blank handling, `-I` lines, quotes spanning lines). `find` now also points at the offending argument in expression errors, with a "did you mean" suggestion for mistyped predicates, and more write errors and malformed arguments are reported instead of panicking.

Nine people made their first contribution to findutils in this cycle: @MsfPablo, @AlejandroCoronadoN, @amoghsingh130, @AbhinavMir, @Jorge-Polanco-Roque, @SichenLiang, @youdie006, @zhangpan2001 and @jayhemnani9910.

We encourage you to support our project by sponsoring us on GitHub. Your sponsorship helps us maintain and enhance our infrastructure, such as GitHub Actions. Sponsor us at [https://github.com/sponsors/uutils](https://github.com/sponsors/uutils).

### GNU Test Suite Compatibility:

| Result        | 0.10.0 | 0.11.0 | Change 0.10.0 to 0.11.0 | % Total 0.10.0 | % Total 0.11.0 | % Change 0.10.0 to 0.11.0 |
|---------------|--------|--------|--------------------------|----------------|----------------|----------------------------|
| Pass          | 417    | 437    | +20                      | 84.24%         | 88.28%         | +4.04%                     |
| Skip          | 1      | 1      | 0                        | 0.20%          | 0.20%          | 0%                         |
| Fail          | 77     | 57     | -20                      | 15.56%         | 11.52%         | -4.04%                     |
| Error         | 0      | 0      | 0                        | 0%             | 0%             | 0%                         |
| Total         | 495    | 495    | 0                        |                |                |                            |

### bfs Test Suite Compatibility:

| Result        | 0.10.0 | 0.11.0 | Change 0.10.0 to 0.11.0 | % Total 0.10.0 | % Total 0.11.0 | % Change 0.10.0 to 0.11.0 |
|---------------|--------|--------|--------------------------|----------------|----------------|----------------------------|
| Pass          | 267    | 280    | +13                      | 85.03%         | 88.33%         | +3.30%                     |
| Skip          | 6      | 6      | 0                        | 1.91%          | 1.89%          | -0.02%                     |
| Fail          | 41     | 31     | -10                      | 13.06%         | 9.78%          | -3.28%                     |
| Total         | 314    | 317    | +3                       |                |                |                            |

The bfs suite gained three tests upstream (317 vs 314).

For more details, visit [https://github.com/uutils/findutils-tracking/](https://github.com/uutils/findutils-tracking/).

## Highlights

### find: `-printf`
* support precision and the `+`, space, `#` and `0` flags; count width and precision in bytes; keep unrecognized directives whole; fail on a precision above `INT_MAX`, by @jayhemnani9910 in https://github.com/uutils/findutils/pull/918
* stop the format at `\c`, by @jayhemnani9910 in https://github.com/uutils/findutils/pull/910 (test: https://github.com/uutils/findutils/pull/919)
* warn on unrecognized directives and escapes and print them literally, instead of dropping or erroring, by @MsfPablo in https://github.com/uutils/findutils/pull/818
* print special bits and no padding in `%m` by @youdie006 in https://github.com/uutils/findutils/pull/891
* pad the day with a space in `%t`, `%a` and `%c` by @jayhemnani9910 in https://github.com/uutils/findutils/pull/917
* write octal escapes as raw bytes by @zhangpan2001 in https://github.com/uutils/findutils/pull/912
* print a trailing backslash by @SichenLiang in https://github.com/uutils/findutils/pull/881

### find: `-ls`
* show the file type and special bits by @youdie006 in https://github.com/uutils/findutils/pull/892
* show allocated blocks and symlink targets by @jayhemnani9910 in https://github.com/uutils/findutils/pull/916
* print times in local time by @jayhemnani9910 in https://github.com/uutils/findutils/pull/922

### find: other GNU compatibility fixes
* `-newerXt`: match only strictly newer files, and keep the sign of pre-1970 times, by @jayhemnani9910 in https://github.com/uutils/findutils/pull/914
* `-newerXt`: parse dates with `parse_datetime`, in local time, by @jayhemnani9910 in https://github.com/uutils/findutils/pull/915
* `-newerXt`: parse ISO 8601 dates by @AlejandroCoronadoN in https://github.com/uutils/findutils/pull/847
* fix symbolic `-perm` matching for directories by @zhangpan2001 in https://github.com/uutils/findutils/pull/931
* preserve `-regextype` across parentheses by @zhangpan2001 in https://github.com/uutils/findutils/pull/934
* reject the deprecated `-perm +MODE` octal form by @AlejandroCoronadoN in https://github.com/uutils/findutils/pull/851
* reject a non-numeric prefix in the `-size` argument by @AlejandroCoronadoN in https://github.com/uutils/findutils/pull/834
* reject a signed `-maxdepth`/`-mindepth` value by @AlejandroCoronadoN in https://github.com/uutils/findutils/pull/835
* output nothing when `-mindepth` exceeds `-maxdepth` by @Jorge-Polanco-Roque in https://github.com/uutils/findutils/pull/857
* stream `-files0-from` input instead of reading it all into memory by @MsfPablo in https://github.com/uutils/findutils/pull/831

### xargs
* only blanks and newlines separate arguments — CR, VT and FF are now kept inside an argument, as in GNU, by @jayhemnani9910 in https://github.com/uutils/findutils/pull/920
* read `-I` input lines like GNU (leading blanks stripped, quoting applied, blank lines skipped, `-E` honored) by @jayhemnani9910 in https://github.com/uutils/findutils/pull/921
* reject quotes spanning input lines by @wtcpython in https://github.com/uutils/findutils/pull/797
* don't emit an empty argument for a trailing blank by @kevinburke in https://github.com/uutils/findutils/pull/829
* retry reading on `BrokenPipe` and use a larger input buffer, so input such as `/dev/kmsg` can be read, by Alessio Attilio (c9ae506, c1aeb11)

### locate / updatedb
* use a platform-dependent default database path and the platform path delimiter by Alessio Attilio (015dc1f)
* only use `std::env::split_paths` on Windows — it panics on other non-Unix targets such as `wasm32-wasip1`, which broke `locate` and `find -execdir` there, by @sylvestre (4357fc7)

### Error messages and diagnostics
* `find`: point at the offending argument in expression errors — when stderr is a terminal (or `UUTILS_DIAG=always`), the argument is underlined and a mistyped predicate gets a "did you mean" suggestion; piped output stays byte-for-byte GNU's single line. By @sylvestre in https://github.com/uutils/findutils/pull/874
* `find`: name the program as invoked in every message by @sylvestre in https://github.com/uutils/findutils/pull/876
* `find`: do not double the `find:` prefix on an invalid `-type` list by @AlejandroCoronadoN in https://github.com/uutils/findutils/pull/849
* `xargs`: strip `(os error N)` from an `--arg-file` open failure by @MsfPablo in https://github.com/uutils/findutils/pull/822
* make `--version` output consistent across utilities by @Xylphy in https://github.com/uutils/findutils/pull/838

### Robustness — errors instead of panics
* `find`: report stdout write errors from `-ls` and `-printf` once, with exit status 1, by @SichenLiang in https://github.com/uutils/findutils/pull/905
* `find`: report `-print` write errors on standard output by @SichenLiang in https://github.com/uutils/findutils/pull/908
* `find`: handle `-fprintf` write errors without panicking by @amoghsingh130 in https://github.com/uutils/findutils/pull/840
* `find`: avoid a panic on a non-UTF-8 argument by @AlejandroCoronadoN in https://github.com/uutils/findutils/pull/833
* `locate`, `updatedb`, `xargs`: avoid a panic on a non-UTF-8 argument by @AbhinavMir in https://github.com/uutils/findutils/pull/853
* `find -printf`: advance past a full character to avoid a multibyte panic by @leeewee in https://github.com/uutils/findutils/pull/731 (test follow-up by @wtcpython in https://github.com/uutils/findutils/pull/870)

### Documentation
* add release notes for every release by @sylvestre in https://github.com/uutils/findutils/pull/805
* document the `-sorted` extension by @sylvestre in https://github.com/uutils/findutils/pull/877 and https://github.com/uutils/findutils/pull/879

### Project & CI
* add `rust-version` to `Cargo.toml` and run CI across several OSes and Rust versions by @xtqqczze in https://github.com/uutils/findutils/pull/808
* remove the deprecated package `authors` field by @xtqqczze in https://github.com/uutils/findutils/pull/823
* add cargo-binstall metadata by @sylvestre in https://github.com/uutils/findutils/pull/814
* sync the release profile with coreutils by @oech3 in https://github.com/uutils/findutils/pull/873
* bump `cargo-dist` from 0.32 to 0.33 by @cakebaker in https://github.com/uutils/findutils/pull/862
* fix an intermittent test failure by @sylvestre in https://github.com/uutils/findutils/pull/844
* clippy: fix warnings by @cakebaker in https://github.com/uutils/findutils/pull/850 and https://github.com/uutils/findutils/pull/888
* ci: build GNU findutils in C17 mode by @sylvestre in https://github.com/uutils/findutils/pull/939
* ci: create the GitHub release as a draft again by @sylvestre in https://github.com/uutils/findutils/pull/938
* prepare version 0.11.0 by @sylvestre in https://github.com/uutils/findutils/pull/872

### Dependencies
Dependency and GitHub Action bumps via Dependabot and manual refreshes by @xtqqczze (#806) and @sylvestre (`uucore` 0.12.0, #875): `uucore` (0.9.0 → 0.12.0, #826/#875), `uutests` (0.9.0 → 0.12.0, #827/#859/#868), `clap` (4.6.4 → 4.6.7, #807/#825/#863), `thiserror` (2.0.19 → 2.0.21, #830/#883), `ctor` (1.0.12 → 1.0.13, #836), `rustix` (1.1.4 → 1.1.5, #882), `rstest` (0.26.1 → 0.28.0, #860/#913), `CodSpeedHQ/action` (5.0.1 → 5.6.1, #810/#824/#856/#895/#901/#909/#924/#940) and `dawidd6/action-download-artifact` (21 → 27, #843/#852/#884/#887/#893).

## New Contributors
* @MsfPablo made their first contribution in https://github.com/uutils/findutils/pull/822
* @AlejandroCoronadoN made their first contribution in https://github.com/uutils/findutils/pull/834
* @amoghsingh130 made their first contribution in https://github.com/uutils/findutils/pull/840
* @AbhinavMir made their first contribution in https://github.com/uutils/findutils/pull/853
* @Jorge-Polanco-Roque made their first contribution in https://github.com/uutils/findutils/pull/857
* @SichenLiang made their first contribution in https://github.com/uutils/findutils/pull/881
* @youdie006 made their first contribution in https://github.com/uutils/findutils/pull/892
* @zhangpan2001 made their first contribution in https://github.com/uutils/findutils/pull/912
* @jayhemnani9910 made their first contribution in https://github.com/uutils/findutils/pull/914

**Full Changelog**: https://github.com/uutils/findutils/compare/0.10.0...0.11.0


## Install findutils 0.11.0

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/uutils/findutils/releases/download/0.11.0/findutils-installer.sh | sh
```

### Install prebuilt binaries via cargo-binstall

```sh
cargo binstall findutils@0.11.0
```

## Download findutils 0.11.0

|  File  | Platform | Checksum |
|--------|----------|----------|
| [findutils-aarch64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-aarch64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-apple-darwin.tar.xz](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-apple-darwin.tar.xz.sha256) |
| [findutils-x86_64-pc-windows-msvc.zip](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-pc-windows-msvc.zip.sha256) |
| [findutils-x86_64-unknown-linux-gnu.tar.xz](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-unknown-linux-gnu.tar.xz.sha256) |
| [findutils-aarch64-unknown-linux-musl.tar.xz](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-aarch64-unknown-linux-musl.tar.xz) | ARM64 MUSL Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-aarch64-unknown-linux-musl.tar.xz.sha256) |
| [findutils-x86_64-unknown-linux-musl.tar.xz](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-unknown-linux-musl.tar.xz) | x64 MUSL Linux | [checksum](https://github.com/uutils/findutils/releases/download/0.11.0/findutils-x86_64-unknown-linux-musl.tar.xz.sha256) |
