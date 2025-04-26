//! SELinux context matcher
//!
//! This matcher will match files based on their
//! SELinux context, only available on Linux.

#[cfg(target_os = "linux")]
use nix::{libc::SELINUX_MAGIC, sys::statvfs::FsFlags};
#[cfg(target_os = "linux")]
use std::{
    error::Error,
    fs::File,
    io::{stderr, BufRead, BufReader, Read, Write},
};

#[cfg(target_os = "linux")]
use super::{glob::Pattern, Matcher, MatcherIO, WalkEntry};

#[cfg(target_os = "linux")]
const XATTR_NAME_SELINUX: &str = "security.selinux";
#[cfg(target_os = "linux")]
const SELINUX_FS: &str = "selinuxfs";
#[cfg(target_os = "linux")]
const SELINUX_MNT: &str = "/sys/fs/selinux";
#[cfg(target_os = "linux")]
const OLD_SELINUX_MNT: &str = "/selinux";

/// Verify if SELinux mount point exists and is writable.
///
/// This function will return true if the SELinux mount point
/// exists and is writable, false otherwise.
#[cfg(target_os = "linux")]
fn verify_selinux_mnt(mnt: &str) -> bool {
    use nix::sys::statfs::{statfs, FsType};
    use nix::sys::statvfs::statvfs;

    let stat = match statfs(mnt) {
        Ok(stat) => stat,
        Err(_) => return false,
    };

    if stat.filesystem_type() == FsType(SELINUX_MAGIC) {
        match statvfs(mnt) {
            Ok(stat) => {
                if stat.flags().contains(FsFlags::ST_RDONLY) {
                    return false;
                }
                return true;
            }
            Err(_) => return false,
        }
    }
    false
}

/// Check if SELinux filesystem exists.
///
/// This function will try to open the `/proc/filesystems` file and
/// check if the SELinux filesystem is listed.
#[cfg(target_os = "linux")]
fn selinuxfs_exists() -> bool {
    let fp = match File::open("/proc/filesystems") {
        Ok(f) => f,
        Err(_) => return true, // Fail as if it exists
    };

    let reader = BufReader::new(fp);
    for line in reader.lines() {
        if let Ok(line) = line {
            if line.contains(SELINUX_FS) {
                return true;
            }
        }
    }
    false
}

/// Get SELinux mount point.
#[cfg(target_os = "linux")]
fn get_selinux_mnt() -> Option<String> {
    if verify_selinux_mnt(SELINUX_MNT) {
        return Some(SELINUX_MNT.to_string());
    }

    if verify_selinux_mnt(OLD_SELINUX_MNT) {
        return Some(OLD_SELINUX_MNT.to_string());
    }

    if !selinuxfs_exists() {
        return None;
    }

    let fp = match File::open("/proc/mounts") {
        Ok(f) => f,
        Err(_) => return None,
    };

    let reader = BufReader::new(fp);
    for line in reader.lines() {
        if let Ok(line) = line {
            let mut parts = line.splitn(3, ' ');
            if let (Some(_), Some(mnt), Some(fs_type)) = (parts.next(), parts.next(), parts.next())
            {
                if fs_type.starts_with(SELINUX_FS) {
                    if verify_selinux_mnt(mnt) {
                        return Some(mnt.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Check if SELinux is enforced.
#[cfg(target_os = "linux")]
fn get_selinux_enforced() -> Result<bool, Box<dyn Error>> {
    let mnt = match get_selinux_mnt() {
        Some(mnt) => mnt,
        None => return Ok(false),
    };

    let path = format!("{mnt}/enforce");
    let mut fd = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(false),
    };

    let mut buf = String::with_capacity(20);
    if fd.read_to_string(&mut buf).is_err() {
        return Ok(false);
    }
    let enforce = i8::from_str_radix(&buf, 10)?;

    Ok(enforce != 0)
}

/// Matcher for SELinux context.
pub struct ContextMatcher {
    #[cfg(target_os = "linux")]
    pattern: Pattern,
}

impl ContextMatcher {
    #[cfg(target_os = "linux")]
    pub fn new(pattern: &str) -> Result<Self, Box<dyn Error>> {
        if !get_selinux_enforced()? {
            return Err(From::from("SELinux is not enabled"));
        }

        let pattern = Pattern::new(pattern, false);

        Ok(Self { pattern })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn new(_pattern: &str) -> Result<Self, Box<dyn Error>> {
        Self {}
    }
}

impl Matcher for ContextMatcher {
    #[cfg(target_os = "linux")]
    fn matches(&self, path: &WalkEntry, _: &mut MatcherIO) -> bool {
        let attr = match xattr::get(path.path(), XATTR_NAME_SELINUX) {
            Ok(attr) => match attr {
                Some(attr) => attr,
                None => {
                    return false;
                }
            },
            Err(e) => {
                writeln!(&mut stderr(), "Failed to get SELinux context: {e}").unwrap();
                return false;
            }
        };
        let selinux_ctx = match String::from_utf8(attr) {
            Ok(selinux_ctx) => selinux_ctx,
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Failed to convert SELinux context to UTF-8: {e}"
                )
                .unwrap();
                return false;
            }
        };
        return self.pattern.matches(&selinux_ctx);
    }

    #[cfg(not(target_os = "linux"))]
    fn matches(&self, _: &WalkEntry, _: &mut MatcherIO) -> bool {
        false
    }
}
