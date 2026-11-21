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
use std::io;
use std::thread::LocalKey;

type NameCache = RefCell<HashMap<u32, String>>;

thread_local! {
    static USER_NAMES: NameCache = RefCell::new(HashMap::new());
    static GROUP_NAMES: NameCache = RefCell::new(HashMap::new());
}

/// The name of user `uid`, or the number when it has no passwd entry, as GNU find shows it.
pub fn user_name(uid: u32) -> String {
    cached(&USER_NAMES, uid, uucore::entries::uid2usr)
}

/// The name of group `gid`, or the number when it has no group entry.
pub fn group_name(gid: u32) -> String {
    cached(&GROUP_NAMES, gid, uucore::entries::gid2grp)
}

fn cached(
    cache: &'static LocalKey<NameCache>,
    id: u32,
    lookup: fn(u32) -> io::Result<String>,
) -> String {
    cache.with_borrow_mut(|names| {
        names
            .entry(id)
            .or_insert_with(|| lookup(id).unwrap_or_else(|_| id.to_string()))
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::unistd::{Gid, Group, Uid, User};

    #[test]
    fn names_match_a_direct_lookup_and_repeat() {
        let uid = Uid::current().as_raw();
        let gid = Gid::current().as_raw();
        let user = uucore::entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string());
        let group = uucore::entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string());

        // The second call is answered from the cache.
        for _ in 0..2 {
            assert_eq!(user_name(uid), user);
            assert_eq!(group_name(gid), group);
        }
    }

    #[test]
    fn an_id_without_an_entry_shows_as_the_number() {
        let uid = (54321..u32::MAX)
            .find(|&id| matches!(User::from_uid(Uid::from_raw(id)), Ok(None)))
            .expect("a uid without a passwd entry");
        let gid = (54321..u32::MAX)
            .find(|&id| matches!(Group::from_gid(Gid::from_raw(id)), Ok(None)))
            .expect("a gid without a group entry");

        for _ in 0..2 {
            assert_eq!(user_name(uid), uid.to_string());
            assert_eq!(group_name(gid), gid.to_string());
        }
    }
}
