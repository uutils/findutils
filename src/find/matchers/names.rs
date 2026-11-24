// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! User and group names for `-ls` and `-printf %u/%g`, and whether an id has an
//! entry at all for `-nouser` and `-nogroup`, remembered per id.
//!
//! Each lookup reads `/etc/passwd` or `/etc/group` (or asks NSS) again, while a
//! tree usually has only a few owners, so each id is looked up once.

use nix::unistd::{Gid, Group, Uid, User};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::thread::LocalKey;

type NameCache = RefCell<HashMap<u32, String>>;
type EntryCache = RefCell<HashMap<u32, bool>>;

thread_local! {
    static USER_NAMES: NameCache = RefCell::new(HashMap::new());
    static GROUP_NAMES: NameCache = RefCell::new(HashMap::new());
    static USER_EXISTS: EntryCache = RefCell::new(HashMap::new());
    static GROUP_EXISTS: EntryCache = RefCell::new(HashMap::new());
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

/// Whether user `uid` has a passwd entry, for `-nouser`.
pub fn user_exists(uid: u32) -> bool {
    exists(&USER_EXISTS, uid, |id| {
        User::from_uid(Uid::from_raw(id)).map(|user| user.is_some())
    })
}

/// Whether group `gid` has a group entry, for `-nogroup`.
pub fn group_exists(gid: u32) -> bool {
    exists(&GROUP_EXISTS, gid, |id| {
        Group::from_gid(Gid::from_raw(id)).map(|group| group.is_some())
    })
}

fn exists(
    cache: &'static LocalKey<EntryCache>,
    id: u32,
    lookup: fn(u32) -> nix::Result<bool>,
) -> bool {
    cache.with_borrow_mut(|known| {
        if let Some(&exists) = known.get(&id) {
            return exists;
        }
        // A failed lookup counts as no entry, as before, but is tried again next time.
        let Ok(exists) = lookup(id) else {
            return false;
        };
        known.insert(id, exists);
        exists
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first uid and gid from 54321 up that have no passwd/group entry.
    fn ids_without_an_entry() -> (u32, u32) {
        let uid = (54321..u32::MAX)
            .find(|&id| matches!(User::from_uid(Uid::from_raw(id)), Ok(None)))
            .expect("a uid without a passwd entry");
        let gid = (54321..u32::MAX)
            .find(|&id| matches!(Group::from_gid(Gid::from_raw(id)), Ok(None)))
            .expect("a gid without a group entry");
        (uid, gid)
    }

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
        let (uid, gid) = ids_without_an_entry();

        for _ in 0..2 {
            assert_eq!(user_name(uid), uid.to_string());
            assert_eq!(group_name(gid), gid.to_string());
        }
    }

    #[test]
    fn existence_matches_a_direct_lookup_and_repeats() {
        let user = matches!(User::from_uid(Uid::current()), Ok(Some(_)));
        let group = matches!(Group::from_gid(Gid::current()), Ok(Some(_)));
        let (no_uid, no_gid) = ids_without_an_entry();

        for _ in 0..2 {
            assert_eq!(user_exists(Uid::current().as_raw()), user);
            assert_eq!(group_exists(Gid::current().as_raw()), group);
            assert!(!user_exists(no_uid));
            assert!(!group_exists(no_gid));
        }
    }
}
