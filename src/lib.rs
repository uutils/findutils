// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub mod find;
pub mod locate;
pub mod updatedb;
pub mod xargs;

use std::ffi::OsStr;
use std::path::PathBuf;

/// Split a `PATH`-style list of paths.
///
/// `std::env::split_paths` only works on Unix and Windows (it panics on WASI and other
/// targets), so outside Windows the list is split on `:` directly.
#[cfg(windows)]
pub(crate) fn split_paths(paths: &OsStr) -> impl Iterator<Item = PathBuf> + '_ {
    std::env::split_paths(paths)
}

#[cfg(not(windows))]
pub(crate) fn split_paths(paths: &OsStr) -> impl Iterator<Item = PathBuf> + '_ {
    paths
        .as_encoded_bytes()
        .split(|&b| b == b':')
        .map(bytes_to_path)
}

/// Decode raw bytes into a path.
///
/// On Unix a path is an arbitrary byte string, so the bytes are used verbatim. On other
/// platforms paths aren't raw bytes, so they are decoded lossily as UTF-8.
#[cfg(unix)]
pub(crate) fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
pub(crate) fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(list: &str) -> Vec<PathBuf> {
        split_paths(OsStr::new(list)).collect()
    }

    #[test]
    fn split_paths_single_entry() {
        assert_eq!(split("/srv/locate.db"), [PathBuf::from("/srv/locate.db")]);
    }

    #[cfg(not(windows))]
    #[test]
    fn split_paths_keeps_empty_entries() {
        assert_eq!(
            split(":/opt/a.db::/var/b.db:"),
            ["", "/opt/a.db", "", "/var/b.db", ""].map(PathBuf::from)
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn split_paths_empty_list() {
        assert_eq!(split(""), [PathBuf::new()]);
    }
}
