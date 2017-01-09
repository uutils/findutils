mod printer;
mod name_matcher;
mod caseless_name_matcher;
mod logical_matchers;
mod type_matcher;
use std::error::Error;
use std::fs::DirEntry;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

/// A basic interface that can be used to determine whether a directory entry
/// is what's being searched for. To a first order approximation, find consists
/// of building a chain of Matcher objets, and then walking a directory tree,
/// passing each entry to the chain of Matchers.
pub trait Matcher {
    /// Returns whether the given file matches the object's predicate.
    fn matches(&self, file_info: &DirEntry) -> bool;

    /// Returns whether the matcher has any side-effects. Iff no such matcher
    /// exists in the chain, then the filename will be printed to stdout. While
    /// this is a compile-time fact for most matchers, it's run-time for matchers
    /// that contain a collection of sub-Matchers.
    fn has_side_effects(&self) -> bool;
}


/// Builds a single AndMatcher containing the Matcher objects corresponding
/// to the passed in predicate arguments.
pub fn build_top_level_matcher(args: &[String],
                               output: Rc<RefCell<Write>>)
                               -> Result<Box<Matcher>, Box<Error>> {
    let mut top_level_matcher = logical_matchers::AndMatcher::new();

    // can't use getopts for a variety or reasons:
    // order ot arguments is important
    // arguments can start with + as well as -
    // multiple-character flags don't start with a double dash
    let mut i = 0;
    while i < args.len() {
        let submatcher = match args[i].as_ref() {
            "-print" => Box::new(printer::Printer::new(output.clone())) as Box<Matcher>,
            "-true" => Box::new(logical_matchers::TrueMatcher {}),
            "-false" => Box::new(logical_matchers::FalseMatcher {}),
            "-name" => {
                i += 1;
                if i >= args.len() {
                    return Err(From::from("Must supply a pattern with -name"));
                }
                Box::new(try!(name_matcher::NameMatcher::new(&args[i])))
            }
            "-iname" => {
                i += 1;
                if i >= args.len() {
                    return Err(From::from("Must supply a pattern with -iname"));
                }
                Box::new(try!(caseless_name_matcher::CaselessNameMatcher::new(&args[i])))
            }
            "-type" => {
                i += 1;
                if i >= args.len() {
                    return Err(From::from("Must supply a type argument with -type"));
                }
                Box::new(try!(type_matcher::TypeMatcher::new(&args[i])))
            }
            _ => return Err(From::from(format!("Unrecognized flag: '{}'", args[i]))),
        };
        top_level_matcher.push(submatcher);
        i += 1;
    }

    if !top_level_matcher.has_side_effects() {
        top_level_matcher.push(Box::new(printer::Printer::new(output)));
    }
    Ok(Box::new(top_level_matcher))
}

#[cfg(test)]
mod tests {
    use std::fs::DirEntry;
    use super::Matcher;

    /// Helper function for tests to get a DirEntry object. directory should
    /// probably be a string starting with "test_data/" (cargo's tests run with
    /// a working directory set to the root findutils folder).
    pub fn get_dir_entry_for(directory: &str, filename: &str) -> DirEntry {
        let dir_entries = ::std::fs::read_dir(directory).unwrap();
        for wrapped_dir_entry in dir_entries {
            let dir_entry = wrapped_dir_entry.unwrap();
            if dir_entry.file_name().to_string_lossy() == filename {
                return dir_entry;
            }
        }
        panic!("Couldn't find {} in {}", directory, filename);
    }

    /// Simple Matcher impl that has side effects
    pub struct HasSideEfects {}

    impl Matcher for HasSideEfects {
        fn matches(&self, _: &DirEntry) -> bool {
            false
        }

        fn has_side_effects(&self) -> bool {
            true
        }
    }

}
