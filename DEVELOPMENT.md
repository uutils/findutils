<!-- spell-checker:ignore (flags) Ccodegen Coverflow Cpanic Zinstrument Zpanic reimplementing toybox RUNTEST CARGOFLAGS nextest prereq autopoint gettext texinfo automake findutils shellenv libexec gnubin toolchains gsed -->

# Setting up your local development environment

For contributing rules and best practices please refer to [CONTRIBUTING.md](CONTRIBUTING.md)

## Before you start

For this guide we assume that you already have a GitHub account and have `git` and your favorite code editor or IDE installed and configured.
Before you start working on findutils, please follow these steps:

1. Fork the [findutils repository](https://github.com/uutils/findutils) to your GitHub account.
***Tip:*** See [this GitHub guide](https://docs.github.com/en/get-started/quickstart/fork-a-repo) for more information on this step.
2. Clone that fork to your local development environment:

```shell
git clone https://github.com/YOUR-GITHUB-ACCOUNT/findutils
cd findutils
```

## Tools

You will need the tools mentioned in this section to build and test your code changes locally.
This section will explain how to install and configure these tools.
We also have an extensive CI that uses these tools and will check your code before it can be merged.
The next section [Testing](#testing) will explain how to run those checks locally to avoid waiting for the CI.

### Rust toolchain

[Install Rust](https://www.rust-lang.org/tools/install)

If you're using rustup to install and manage your Rust toolchains, `clippy` and `rustfmt` are usually already installed. If you are using one of the alternative methods, please make sure to install them manually. See following sub-sections for their usage: [clippy](#clippy) [rustfmt](#rustfmt).

***Tip*** You might also need to add 'llvm-tools' component if you are going to [generate code coverage reports locally](#code-coverage-report):

```shell
rustup component add llvm-tools-preview
```

### pre-commit hooks

A configuration for `pre-commit` is provided in the repository. It allows
automatically checking every git commit you make to ensure it compiles, and
passes `clippy` and `rustfmt` without warnings.

To use the provided hook:

1. [Install `pre-commit`](https://pre-commit.com/#install)
1. Run `pre-commit install` while in the repository directory

Your git commits will then automatically be checked. If a check fails, an error
message will explain why, and your commit will be canceled. You can then make
the suggested changes, and run `git commit ...` again.

**NOTE: On MacOS** the pre-commit hooks are currently broken. There are workarounds involving switching to unstable nightly Rust and components.

### clippy

```shell
cargo clippy --all-targets --all-features
```

The `msrv` key in the clippy configuration file `clippy.toml` is used to disable
lints pertaining to newer features by specifying the minimum supported Rust
version (MSRV).

### rustfmt

```shell
cargo fmt --all
```

### cargo-deny

This project uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny/) to
detect duplicate dependencies, checks licenses, etc. To run it locally, first
install it and then run with:

```shell
cargo deny --all-features check all
```

### Markdown linter

We use [markdownlint](https://github.com/DavidAnson/markdownlint) to lint the
Markdown files in the repository.

### Spell checker

We use `cspell` as spell checker for all files in the project. If you are using
VS Code, you can install the
[code spell checker](https://marketplace.visualstudio.com/items?itemName=streetsidesoftware.code-spell-checker)
extension to enable spell checking within your editor. Otherwise, you can
install [cspell](https://cspell.org/) separately.

If you want to make the spell checker ignore a word, you can add

```rust
// spell-checker:ignore word_to_ignore
```

at the top of the file.

## Testing

Just like with building, we follow the standard procedure for testing using
Cargo:

```shell
cargo test
```

## Code coverage report

Code coverage report can be generated using [grcov](https://github.com/mozilla/grcov).

### Using Nightly Rust

To generate [gcov-based](https://github.com/mozilla/grcov#example-how-to-generate-gcda-files-for-a-rust-project) coverage report

```shell
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
cargo build <options...>
cargo test <options...>
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing --ignore build.rs --excl-br-line "^\s*((debug_)?assert(_eq|_ne)?\#\[derive\()" -o ./target/debug/coverage/
# open target/debug/coverage/index.html in browser
```

if changes are not reflected in the report then run `cargo clean` and run the above commands.

### Using Stable Rust

If you are using stable version of Rust that doesn't enable code coverage instrumentation by default
then add `-Z-Zinstrument-coverage` flag to `RUSTFLAGS` env variable specified above.

## Tips for setting up on Mac

### C Compiler and linker

On MacOS you'll need to install C compiler & linker:

```shell
xcode-select --install
```

## Tips for setting up on Windows

### MSVC build tools

On Windows you'll need the MSVC build tools for Visual Studio 2013 or later.

If you are using `rustup-init.exe` to install Rust toolchain, it will guide you through the process of downloading and installing these prerequisites.

Otherwise please follow [this guide](https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup).
