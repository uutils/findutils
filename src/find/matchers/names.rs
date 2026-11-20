// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! User and group names for `-ls` and `-printf %u/%g`, remembered per id.
//!
//! Each lookup reads `/etc/passwd` or `/etc/group` (or asks NSS) again, while a
//! tree usually has only a few owners, so the name is looked up once per id.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static USER_NAMES: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
    static GROUP_NAMES: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
}

/// The name of user `uid`, or the number when it has no passwd entry, as GNU find shows it.
pub fn user_name(uid: u32) -> String {
    USER_NAMES.with_borrow_mut(|names| {
        names
            .entry(uid)
            .or_insert_with(|| uucore::entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()))
            .clone()
    })
}

/// The name of group `gid`, or the number when it has no group entry.
pub fn group_name(gid: u32) -> String {
    GROUP_NAMES.with_borrow_mut(|names| {
        names
            .entry(gid)
            .or_insert_with(|| uucore::entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()))
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_a_direct_lookup_and_repeat() {
        let uid = unsafe { uucore::libc::getuid() };
        let gid = unsafe { uucore::libc::getgid() };
        let user = uucore::entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string());
        let group = uucore::entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string());

        assert_eq!(user_name(uid), user);
        assert_eq!(user_name(uid), user);
        assert_eq!(group_name(gid), group);
        assert_eq!(group_name(gid), group);
    }
}
