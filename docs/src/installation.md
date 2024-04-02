<!-- spell-checker:ignore pacman pamac nixpkgs openmandriva conda winget openembedded yocto bblayers bitbake -->

# Installation

This is a list of uutils packages in various distributions and package managers.
Note that these are packaged by third-parties and the packages might contain
patches.

You can also [build findutils from source](build.md).

<!-- toc -->

## Cargo

[![crates.io package](https://repology.org/badge/version-for-repo/crates_io/rust:findutils.svg)](https://crates.io/crates/findutils)

```shell
cargo install findutils
```

## Linux

### Debian

[![Debian 13 package](https://repology.org/badge/version-for-repo/debian_13/rust:findutils.svg)](https://packages.debian.org/trixie/source/rust-findutils)

[![Debian Unstable package](https://repology.org/badge/version-for-repo/debian_unstable/rust:findutils.svg)](https://packages.debian.org/sid/source/rust-findutils)

```shell
apt install rust-findutils
# To use it:
export PATH=/usr/lib/cargo/bin/findutils:$PATH
```

### Gentoo

[![Gentoo package](https://repology.org/badge/version-for-repo/gentoo/uutils-findutils.svg)](https://packages.gentoo.org/packages/sys-apps/uutils-findutils)

```shell
emerge -pv sys-apps/uutils-findutils
```

## MacOS

### Homebrew

[![Homebrew package](https://repology.org/badge/version-for-repo/homebrew/uutils-findutils.svg)](https://formulae.brew.sh/formula/uutils-findutils)

```shell
brew install uutils-findutils
```


## FreeBSD

[![FreeBSD port](https://repology.org/badge/version-for-repo/freebsd/rust-findutils.svg)](https://repology.org/project/rust-findutils/versions)

```sh
pkg install rust-findutils
```

## Windows

As far as we are aware, `findutils` has not been packaged for any package managers on Windows yet.