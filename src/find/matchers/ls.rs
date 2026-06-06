// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::DateTime;
use std::{
    fs::File,
    io::{stderr, Write},
};

use super::{Matcher, MatcherIO, WalkEntry};

#[cfg(unix)]
fn format_permissions(mode: uucore::libc::mode_t) -> String {
    let file_type = match mode & (uucore::libc::S_IFMT as uucore::libc::mode_t) {
        uucore::libc::S_IFDIR => "d",
        uucore::libc::S_IFREG => "-",
        _ => "?",
    };

    // S_$$USR means "user permissions"
    let user_perms = format!(
        "{}{}{}",
        if mode & uucore::libc::S_IRUSR != 0 {
            "r"
        } else {
            "-"
        },
        if mode & uucore::libc::S_IWUSR != 0 {
            "w"
        } else {
            "-"
        },
        if mode & uucore::libc::S_IXUSR != 0 {
            "x"
        } else {
            "-"
        }
    );

    // S_$$GRP means "group permissions"
    let group_perms = format!(
        "{}{}{}",
        if mode & uucore::libc::S_IRGRP != 0 {
            "r"
        } else {
            "-"
        },
        if mode & uucore::libc::S_IWGRP != 0 {
            "w"
        } else {
            "-"
        },
        if mode & uucore::libc::S_IXGRP != 0 {
            "x"
        } else {
            "-"
        }
    );

    // S_$$OTH means "other permissions"
    let other_perms = format!(
        "{}{}{}",
        if mode & uucore::libc::S_IROTH != 0 {
            "r"
        } else {
            "-"
        },
        if mode & uucore::libc::S_IWOTH != 0 {
            "w"
        } else {
            "-"
        },
        if mode & uucore::libc::S_IXOTH != 0 {
            "x"
        } else {
            "-"
        }
    );

    format!("{}{}{}{}", file_type, user_perms, group_perms, other_perms)
}

#[cfg(windows)]
fn format_permissions(file_attributes: u32) -> String {
    // https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
    const FILE_ATTRIBUTE_READONLY: u32 = 0x0001;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

    let file_type = if file_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        "l"
    } else if file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        "d"
    } else {
        "-"
    };

    let write = if file_attributes & FILE_ATTRIBUTE_READONLY != 0 {
        "-"
    } else {
        "w"
    };

    format!("{file_type}r{write}xr-xr-x")
}

#[cfg(windows)]
fn lookup_owner_group(path: &std::path::Path) -> Option<(String, String)> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    };

    fn account_name_from_sid(sid: PSID) -> Option<String> {
        use std::ptr::{null, null_mut};
        use windows_sys::Win32::Security::LookupAccountSidW;

        fn wide_to_string(buf: &[u16], len: u32) -> String {
            let len = (len as usize).min(buf.len());
            let mut slice = &buf[..len];
            if slice.last() == Some(&0) {
                slice = &slice[..slice.len() - 1];
            }
            String::from_utf16_lossy(slice)
        }

        if sid.is_null() {
            return None;
        }

        let mut name_len = 0;
        let mut domain_len = 0;
        let mut sid_name_use = 0;

        unsafe {
            LookupAccountSidW(
                null(),
                sid,
                null_mut(),
                &mut name_len,
                null_mut(),
                &mut domain_len,
                &mut sid_name_use,
            );
        }

        if name_len == 0 {
            return None;
        }

        let mut name = vec![0; name_len as usize];
        let mut domain = vec![0; domain_len as usize];
        let ok = unsafe {
            LookupAccountSidW(
                null(),
                sid,
                name.as_mut_ptr(),
                &mut name_len,
                if domain.is_empty() {
                    null_mut()
                } else {
                    domain.as_mut_ptr()
                },
                &mut domain_len,
                &mut sid_name_use,
            )
        };

        if ok == 0 {
            return None;
        }

        let name = wide_to_string(&name, name_len);
        let domain = wide_to_string(&domain, domain_len);
        if domain.is_empty() {
            Some(name)
        } else {
            Some(format!("{domain}\\{name}"))
        }
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut owner_sid: PSID = null_mut();
    let mut group_sid: PSID = null_mut();
    let mut security_descriptor: PSECURITY_DESCRIPTOR = null_mut();

    let status = unsafe {
        GetNamedSecurityInfoW(
            wide_path.as_mut_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            &mut owner_sid,
            &mut group_sid,
            null_mut(),
            null_mut(),
            &mut security_descriptor,
        )
    };

    if status != 0 {
        return None;
    }

    let owner = account_name_from_sid(owner_sid).unwrap_or_else(|| "0".to_string());
    let group = account_name_from_sid(group_sid).unwrap_or_else(|| "0".to_string());

    unsafe {
        if !security_descriptor.is_null() {
            LocalFree(security_descriptor);
        }
    }

    Some((owner, group))
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
        use nix::unistd::{Gid, Group, Uid, User};
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
        let permission =
            { format_permissions(metadata.permissions().mode() as uucore::libc::mode_t) };
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

    #[cfg(windows)]
    fn print(
        &self,
        file_info: &WalkEntry,
        matcher_io: &mut MatcherIO,
        mut out: impl Write,
        print_error_message: bool,
    ) {
        use std::os::windows::fs::MetadataExt;

        let metadata = file_info.metadata().unwrap();

        let inode_number = 0;
        let number_of_blocks = {
            let size = metadata.file_size();
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
        let permission = { format_permissions(metadata.file_attributes()) };
        let hard_links = 0;
        let (user, group) = lookup_owner_group(file_info.path())
            .unwrap_or_else(|| ("0".to_string(), "0".to_string()));
        let size = metadata.file_size();
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
    #[test]
    #[cfg(unix)]
    fn test_format_permissions() {
        use super::format_permissions;

        let mode: uucore::libc::mode_t = 0o100644;
        let expected = "-rw-r--r--";
        assert_eq!(format_permissions(mode), expected);

        let mode: uucore::libc::mode_t = 0o040755;
        let expected = "drwxr-xr-x";
        assert_eq!(format_permissions(mode), expected);

        let mode: uucore::libc::mode_t = 0o100777;
        let expected = "-rwxrwxrwx";
        assert_eq!(format_permissions(mode), expected);
    }

    #[test]
    #[cfg(windows)]
    fn test_format_permissions() {
        use super::format_permissions;

        const FILE_ATTRIBUTE_READONLY: u32 = 0x0001;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

        assert_eq!(format_permissions(0), "-rwxr-xr-x");
        assert_eq!(format_permissions(FILE_ATTRIBUTE_READONLY), "-r-xr-xr-x");
        assert_eq!(format_permissions(FILE_ATTRIBUTE_DIRECTORY), "drwxr-xr-x");
        assert_eq!(
            format_permissions(FILE_ATTRIBUTE_REPARSE_POINT),
            "lrwxr-xr-x"
        );
    }
}
