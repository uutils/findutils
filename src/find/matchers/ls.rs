// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::DateTime;
use std::{
    fs::File,
    io::{stderr, Write},
    time::SystemTime,
};

use super::{Matcher, MatcherIO, WalkEntry};

/// Formats the date column: the time of day for
/// timestamps from 180 days before find started until one hour after it,
/// comparing whole seconds, and the year for anything else.
fn format_date(modified: SystemTime, now: SystemTime) -> impl std::fmt::Display {
    const RECENT_PAST_SECS: i64 = 180 * 24 * 60 * 60;
    const RECENT_FUTURE_SECS: i64 = 60 * 60;

    let modified: DateTime<chrono::Utc> = modified.into();
    let now = DateTime::<chrono::Utc>::from(now).timestamp();
    let recent = now - RECENT_PAST_SECS..=now + RECENT_FUTURE_SECS;
    let format = if recent.contains(&modified.timestamp()) {
        "%b %e %H:%M"
    } else {
        "%b %e  %Y"
    };
    modified.format(format)
}

#[cfg(windows)]
fn format_permissions(file_attributes: u32) -> String {
    let mut attributes = Vec::new();

    // https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
    if file_attributes & 0x0001 != 0 {
        attributes.push("read-only");
    }
    if file_attributes & 0x0002 != 0 {
        attributes.push("hidden");
    }
    if file_attributes & 0x0004 != 0 {
        attributes.push("system");
    }
    if file_attributes & 0x0020 != 0 {
        attributes.push("archive");
    }
    if file_attributes & 0x0040 != 0 {
        attributes.push("compressed");
    }
    if file_attributes & 0x0080 != 0 {
        attributes.push("offline");
    }

    attributes.join(", ")
}

pub struct Ls {
    output_file: Option<File>,
}

impl Ls {
    pub fn new(output_file: Option<File>) -> Self {
        Self { output_file }
    }

    #[cfg(unix)]
    fn print(
        &self,
        file_info: &WalkEntry,
        matcher_io: &mut MatcherIO,
        mut out: impl Write,
        print_error_message: bool,
    ) {
        use std::os::unix::fs::MetadataExt;

        let metadata = file_info.metadata().unwrap();

        let inode_number = metadata.ino();
        let number_of_blocks = {
            let size = metadata.size();
            let number_of_blocks = size / 1024;
            let remainder = number_of_blocks % 4;

            if remainder == 0 {
                if number_of_blocks == 0 {
                    4
                } else {
                    number_of_blocks
                }
            } else {
                number_of_blocks + (4 - (remainder))
            }
        };
        let permission = uucore::fs::display_permissions(metadata, true);
        let hard_links = metadata.nlink();
        // Fall back to the numeric id when the uid/gid has no passwd/group entry
        // (unmapped owner) — matching GNU find, which never crashes. uucore::entries
        // serializes the non-thread-safe getpwuid/getgrgid behind a mutex.
        let user =
            uucore::entries::uid2usr(metadata.uid()).unwrap_or_else(|_| metadata.uid().to_string());
        let group =
            uucore::entries::gid2grp(metadata.gid()).unwrap_or_else(|_| metadata.gid().to_string());
        let size = metadata.size();
        let last_modified = format_date(metadata.modified().unwrap(), matcher_io.now());
        let path = file_info.path().to_string_lossy();

        match writeln!(
            out,
            " {:<4} {:>6} {:<10} {:>3} {:<8} {:<8} {:>8} {} {}",
            inode_number,
            number_of_blocks,
            permission,
            hard_links,
            user,
            group,
            size,
            last_modified,
            path,
        ) {
            Ok(_) => {}
            Err(e) => {
                if print_error_message {
                    writeln!(
                        &mut stderr(),
                        "Error writing {:?} for {}",
                        file_info.path().to_string_lossy(),
                        e
                    )
                    .unwrap();
                    matcher_io.set_exit_code(1);
                }
            }
        }
    }

    #[cfg(not(unix))]
    fn print(
        &self,
        file_info: &WalkEntry,
        matcher_io: &mut MatcherIO,
        mut out: impl Write,
        print_error_message: bool,
    ) {
        // Non-Unix targets (Windows, wasm, ...) don't expose inode, owner,
        // group or Unix permission bits, so those columns are left blank.
        let metadata = file_info.metadata().unwrap();

        let inode_number = 0;
        let size = metadata.len();
        let number_of_blocks = {
            let number_of_blocks = size / 1024;
            let remainder = number_of_blocks % 4;
            if remainder == 0 {
                if number_of_blocks == 0 {
                    4
                } else {
                    number_of_blocks
                }
            } else {
                number_of_blocks + (4 - remainder)
            }
        };
        #[cfg(windows)]
        let permission = {
            use std::os::windows::fs::MetadataExt;
            format_permissions(metadata.file_attributes())
        };
        #[cfg(not(windows))]
        let permission = "?---------";
        let hard_links = 0;
        let user = 0;
        let group = 0;
        let last_modified = format_date(metadata.modified().unwrap(), matcher_io.now());
        let path = file_info.path().to_string_lossy();

        match writeln!(
            out,
            " {:<4} {:>6} {:<10} {:>3} {:<8} {:<8} {:>8} {} {}",
            inode_number,
            number_of_blocks,
            permission,
            hard_links,
            user,
            group,
            size,
            last_modified,
            path,
        ) {
            Ok(_) => {}
            Err(e) => {
                if print_error_message {
                    writeln!(
                        &mut stderr(),
                        "Error writing {:?} for {}",
                        file_info.path().to_string_lossy(),
                        e
                    )
                    .unwrap();
                    matcher_io.set_exit_code(1);
                }
            }
        }
    }
}

impl Matcher for Ls {
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        if let Some(file) = &self.output_file {
            self.print(file_info, matcher_io, file, true);
        } else {
            self.print(
                file_info,
                matcher_io,
                &mut *matcher_io.deps.get_output().borrow_mut(),
                false,
            );
        }
        true
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::format_date;

    #[test]
    fn date_shows_year_outside_recent_window() {
        // find started at 2023-11-14 22:13:20.9 UTC.
        let now_secs = 1_700_000_000;
        let now = UNIX_EPOCH + Duration::new(now_secs, 900_000_000);
        let date =
            |secs, nanos| format_date(UNIX_EPOCH + Duration::new(secs, nanos), now).to_string();

        assert_eq!(date(now_secs - 15_552_000, 0), "May 18 22:13");
        assert_eq!(date(now_secs - 15_552_001, 999_999_999), "May 18  2023");
        assert_eq!(date(now_secs + 3_600, 500_000_000), "Nov 14 23:13");
        assert_eq!(date(now_secs + 3_601, 0), "Nov 14  2023");
        assert_eq!(
            format_date(UNIX_EPOCH - Duration::from_secs(1), now).to_string(),
            "Dec 31  1969"
        );
    }
}
