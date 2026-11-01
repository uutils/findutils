// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::DateTime;
use std::{
    fs::File,
    io::{stderr, Write},
};

use super::{Matcher, MatcherIO, StdoutErrors, WalkEntry};

#[cfg(windows)]
fn format_permissions(is_symlink: bool, file_attributes: u32) -> String {
    use uucore::windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
    };

    // The link check has to come first: a link to a directory also carries
    // FILE_ATTRIBUTE_DIRECTORY. Only true symlinks print `l`: junctions stay
    // `d` and OneDrive placeholders `-`, none of which are symlinks.
    let file_type = if is_symlink {
        "l"
    } else if file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        "d"
    } else {
        "-"
    };

    // Windows ACLs have no POSIX mode bits for this column to print, so
    // synthesize a fixed approximation rather than list attribute names:
    // * `r` is always set: read access cannot be derived from the metadata
    //   find reads, only from the full ACL;
    // * `w` mirrors FILE_ATTRIBUTE_READONLY, the only write-related bit;
    // * `x` is always set: Windows has no execute bit, every file is
    //   potentially executable;
    // * group and other mirror the owner as `r-x`: telling them apart would
    //   require walking the DACL per file.
    let write = if file_attributes & FILE_ATTRIBUTE_READONLY != 0 {
        "-"
    } else {
        "w"
    };

    format!("{file_type}r{write}xr-xr-x")
}

/// Whether `path` is a reparse point with the symlink tag. `std` reports
/// every name-surrogate reparse point through `FileType::is_symlink`, which
/// includes junctions, so the tag itself has to be checked. Only reparse
/// points (junctions, links, OneDrive placeholders, ...) pay for the extra
/// `FindFirstFileW`, which exposes the tag in `dwReserved0`.
#[cfg(windows)]
fn is_symlink_reparse_point(path: &std::path::Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    use uucore::windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use uucore::windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstFileW, FILE_ATTRIBUTE_REPARSE_POINT, WIN32_FIND_DATAW,
    };

    // windows-sys puts this tag behind its Win32_System_SystemServices
    // feature, which uucore does not enable.
    const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;

    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut find_data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    // SAFETY: wide_path is null-terminated and find_data a valid output
    // buffer.
    let handle = unsafe { FindFirstFileW(wide_path.as_ptr(), &raw mut find_data) };
    if handle == INVALID_HANDLE_VALUE {
        return false;
    }
    // SAFETY: handle comes from FindFirstFileW above.
    unsafe {
        FindClose(handle);
    }
    find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        && find_data.dwReserved0 == IO_REPARSE_TAG_SYMLINK
}

/// Get the owner and group of `path` for display: `DOMAIN\name` when each SID
/// resolves to an account, the raw `S-1-...` SID string when it does not (e.g.
/// a deleted account), or the `0` placeholder when not even the security
/// descriptor could be read.
#[cfg(windows)]
fn display_owner_group(path: &std::path::Path) -> (String, String) {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};

    use uucore::windows_sys::Win32::Security::{
        GetFileSecurityW, GetLengthSid, GetSecurityDescriptorGroup, GetSecurityDescriptorOwner,
        LookupAccountSidW, GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, PSID,
    };

    thread_local! {
        /// SIDs resolved so far, failures included: one account lookup per
        /// distinct SID per run instead of one per file, so a SID without an
        /// account cannot reach the domain controller for every file it
        /// appears on. `find` walks the tree single-threaded, so the cache
        /// needs no locking.
        static SID_NAMES: RefCell<HashMap<Box<[u8]>, Option<String>>> =
            RefCell::new(HashMap::new());
    }

    /// Look up `sid` in the cache, filling it with `LookupAccountSidW` on a
    /// miss (classic two-call size query, then fill).
    fn display_name(sid: PSID) -> String {
        fn resolve_account(sid: PSID) -> Option<String> {
            let mut name_len = 0;
            let mut domain_len = 0;
            let mut sid_type = 0;
            // SAFETY: name and domain buffers may be null when only querying
            // the required lengths.
            unsafe {
                LookupAccountSidW(
                    null(),
                    sid,
                    null_mut(),
                    &raw mut name_len,
                    null_mut(),
                    &raw mut domain_len,
                    &raw mut sid_type,
                );
            }
            if name_len == 0 {
                return None;
            }
            let mut name = vec![0; name_len as usize];
            let mut domain = vec![0; domain_len as usize];
            // SAFETY: buffers are sized from the preceding length query.
            let ok = unsafe {
                LookupAccountSidW(
                    null(),
                    sid,
                    name.as_mut_ptr(),
                    &raw mut name_len,
                    if domain.is_empty() {
                        null_mut()
                    } else {
                        domain.as_mut_ptr()
                    },
                    &raw mut domain_len,
                    &raw mut sid_type,
                )
            };
            if ok == 0 {
                return None;
            }
            let name = String::from_utf16_lossy(&name[..(name_len as usize).min(name.len())]);
            let domain =
                String::from_utf16_lossy(&domain[..(domain_len as usize).min(domain.len())]);
            if domain.is_empty() {
                Some(name)
            } else {
                Some(format!("{domain}\\{name}"))
            }
        }

        /// Format a SID as its canonical `S-1-...` string. windows-sys only
        /// exposes `ConvertSidToStringSidW` behind the
        /// `Win32_Security_Authorization` feature, and the format is plain:
        /// revision, identifier authority, then the sub-authorities.
        fn sid_to_string(sid: PSID) -> Option<String> {
            // SAFETY: SID layout — byte 0 is the revision, byte 1 the
            // sub-authority count, bytes 2..8 the big-endian identifier
            // authority, then `count` little-endian u32 sub-authorities.
            let raw = sid.cast::<u8>();
            let (revision, count, authority) = unsafe {
                (
                    raw.read(),
                    raw.add(1).read(),
                    u64::from_be_bytes([
                        raw.add(2).read(),
                        raw.add(3).read(),
                        raw.add(4).read(),
                        raw.add(5).read(),
                        raw.add(6).read(),
                        raw.add(7).read(),
                        0,
                        0,
                    ]),
                )
            };
            if count == 0 || count > 15 {
                return None;
            }
            let mut result = format!("S-{revision}-{authority}");
            for i in 0..count as usize {
                // SAFETY: sub-authority i lives at bytes 8 + 4*i..12 + 4*i.
                // The SID may sit in a byte buffer, so read unaligned.
                let subauthority = unsafe { raw.add(8 + i * 4).cast::<u32>().read_unaligned() };
                result.push_str(&format!("-{subauthority}"));
            }
            Some(result)
        }

        if sid.is_null() {
            return "0".to_string();
        }
        // SAFETY: sid points to a valid SID of GetLengthSid(sid) bytes.
        let key = unsafe {
            Box::from(std::slice::from_raw_parts(
                sid.cast::<u8>(),
                GetLengthSid(sid) as usize,
            ))
        };
        if let Some(cached) = SID_NAMES.with_borrow(|cache| cache.get(&key).cloned()) {
            return cached.unwrap_or_else(|| sid_to_string(sid).unwrap_or_else(|| "0".into()));
        }
        let name = resolve_account(sid);
        SID_NAMES.with_borrow_mut(|cache| cache.insert(key, name.clone()));
        name.unwrap_or_else(|| sid_to_string(sid).unwrap_or_else(|| "0".into()))
    }

    let information = OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION;
    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut needed = 0;
    // SAFETY: passing a null descriptor queries the required buffer size.
    unsafe {
        GetFileSecurityW(
            wide_path.as_ptr(),
            information,
            null_mut(),
            0,
            &raw mut needed,
        );
    }
    if needed == 0 {
        return ("0".to_string(), "0".to_string());
    }
    let mut buffer = vec![0u8; needed as usize];
    // SAFETY: the self-relative descriptor lives in `buffer` for the rest of
    // this function, so nothing needs to be freed.
    let descriptor = buffer.as_mut_ptr() as PSECURITY_DESCRIPTOR;
    // SAFETY: descriptor has room for the `needed` bytes reported above.
    let ok = unsafe {
        GetFileSecurityW(
            wide_path.as_ptr(),
            information,
            descriptor,
            needed,
            &raw mut needed,
        )
    };
    if ok == 0 {
        return ("0".to_string(), "0".to_string());
    }
    let mut owner: PSID = null_mut();
    let mut group: PSID = null_mut();
    let mut defaulted = 0;
    // SAFETY: descriptor is a valid security descriptor from the call above.
    unsafe {
        GetSecurityDescriptorOwner(descriptor, &raw mut owner, &raw mut defaulted);
        GetSecurityDescriptorGroup(descriptor, &raw mut group, &raw mut defaulted);
    }
    (display_name(owner), display_name(group))
}

pub struct Ls {
    output_file: Option<File>,
    stdout_errors: StdoutErrors,
}

impl Ls {
    pub fn new(output_file: Option<File>) -> Self {
        Self {
            output_file,
            stdout_errors: StdoutErrors::default(),
        }
    }

    pub fn with_stdout_errors(mut self, stdout_errors: StdoutErrors) -> Self {
        self.stdout_errors = stdout_errors;
        self
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
        // st_blocks counts 512-byte units; -ls shows 1 KiB ones
        let number_of_blocks = metadata.blocks().div_ceil(2);
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
        let last_modified = {
            let system_time = metadata.modified().unwrap();
            let local: DateTime<chrono::Local> = system_time.into();
            local.format("%b %e %H:%M")
        };
        let mut path = file_info.path().to_string_lossy().into_owned();
        if file_info.file_type().is_symlink() {
            if let Ok(target) = std::fs::read_link(file_info.path()) {
                path.push_str(" -> ");
                path.push_str(&target.to_string_lossy());
            }
        }

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
                } else {
                    self.stdout_errors.handle(&e, matcher_io);
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
        // Non-Unix targets have no inode number. Windows fills the remaining
        // columns from its own metadata, other targets keep placeholders.
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
            // path_is_symlink() alone would also flag junctions, so confirm
            // the reparse tag for those entries.
            let is_symlink =
                file_info.path_is_symlink() && is_symlink_reparse_point(file_info.path());
            format_permissions(is_symlink, metadata.file_attributes())
        };
        #[cfg(not(windows))]
        let permission = "?---------";
        let hard_links = 0;
        #[cfg(windows)]
        let (user, group) = display_owner_group(file_info.path());
        #[cfg(not(windows))]
        let (user, group) = ("0".to_string(), "0".to_string());
        let last_modified = {
            let system_time = metadata.modified().unwrap();
            let local: DateTime<chrono::Local> = system_time.into();
            local.format("%b %e %H:%M")
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
                } else {
                    self.stdout_errors.handle(&e, matcher_io);
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

#[cfg(all(test, windows))]
mod tests {
    use super::format_permissions;
    use uucore::windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT,
    };

    #[test]
    fn test_format_permissions() {
        assert_eq!(format_permissions(false, 0), "-rwxr-xr-x");
        assert_eq!(
            format_permissions(false, FILE_ATTRIBUTE_READONLY),
            "-r-xr-xr-x"
        );
        assert_eq!(
            format_permissions(false, FILE_ATTRIBUTE_DIRECTORY),
            "drwxr-xr-x"
        );
        // A link to a directory carries both attributes; the link wins.
        assert_eq!(
            format_permissions(
                true,
                FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT
            ),
            "lrwxr-xr-x"
        );
    }
}
