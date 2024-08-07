// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::DateTime;
use nix::unistd::{Gid, Group, Uid, User};
use std::{
    fs::File,
    io::{stderr, Write},
};
use walkdir::DirEntry;

use super::{Matcher, MatcherIO};

#[cfg(unix)]
fn format_permissions(mode: u32) -> String {
    let file_type = if mode & 0o170000 == 0o040000 {
        "d"
    } else if mode & 0o170000 == 0o100000 {
        "-"
    } else {
        "?"
    };

    let user_perms = format!(
        "{}{}{}",
        if mode & 0o0400 != 0 { "r" } else { "-" },
        if mode & 0o0200 != 0 { "w" } else { "-" },
        if mode & 0o0100 != 0 { "x" } else { "-" }
    );

    let group_perms = format!(
        "{}{}{}",
        if mode & 0o0040 != 0 { "r" } else { "-" },
        if mode & 0o0020 != 0 { "w" } else { "-" },
        if mode & 0o0010 != 0 { "x" } else { "-" }
    );

    let other_perms = format!(
        "{}{}{}",
        if mode & 0o0004 != 0 { "r" } else { "-" },
        if mode & 0o0002 != 0 { "w" } else { "-" },
        if mode & 0o0001 != 0 { "x" } else { "-" }
    );

    format!("{}{}{}{}", file_type, user_perms, group_perms, other_perms)
}

pub struct Ls {
    output_file: Option<File>,
}

impl Ls {
    pub fn new(output_file: Option<File>) -> Self {
        Self { output_file }
    }

    #[cfg(unix)]
    fn print(&self, file_info: &DirEntry, mut out: impl Write, print_error_message: bool) {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

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
        let permission = { format_permissions(metadata.permissions().mode()) };
        let hard_links = metadata.nlink();
        let user = {
            let uid = metadata.uid();
            User::from_uid(Uid::from_raw(uid)).unwrap().unwrap().name
        };
        let group = {
            let gid = metadata.gid();
            Group::from_gid(Gid::from_raw(gid)).unwrap().unwrap().name
        };
        let size = metadata.size();
        let last_modified = {
            let system_time = metadata.modified().unwrap();
            let now_utc: DateTime<chrono::Utc> = system_time.into();
            now_utc.format("%b %e %H:%M")
        };
        let path = file_info.path().to_string_lossy();

        match write!(
            out,
            " {:<4} {:>6} {:<10} {:>3} {:<8} {:<8} {:>8} {} {}\n",
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
                    uucore::error::set_exit_code(1);
                }
            }
        }
    }

    #[cfg(windows)]
    fn print(&self, file_info: &DirEntry, mut out: impl Write, print_error_message: bool) {}
}

impl Matcher for Ls {
    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        if let Some(file) = &self.output_file {
            self.print(file_info, file, true);
        } else {
            self.print(
                file_info,
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
