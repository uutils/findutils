use std::{
    error::Error,
    io::{stderr, Write},
};

use super::{glob::Pattern, Matcher, MatcherIO, WalkEntry};

pub struct ContextMatcher {
    pattern: Pattern,
}

impl ContextMatcher {
    pub fn new(pattern: &str) -> Result<Self, Box<dyn Error>> {
        let pattern = Pattern::new(pattern, false);

        Ok(Self { pattern })
    }
}

impl Matcher for ContextMatcher {
    fn matches(&self, path: &WalkEntry, _: &mut MatcherIO) -> bool {
        let attr = match xattr::get(path.path(), "security.selinux") {
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
                writeln!(&mut stderr(), "Failed to convert SELinux context to UTF-8: {e}").unwrap();
                return false;
            }
        };
        return self.pattern.matches(&selinux_ctx);
    }
}
