// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub mod matchers;

use std::cell::RefCell;
use std::error::Error;
use std::io::{stderr, stdout, Write};
use std::rc::Rc;
use std::time::SystemTime;
use walkdir::WalkDir;

pub struct Config {
    depth_first: bool,
    min_depth: usize,
    max_depth: usize,
    sorted_output: bool,
    help_requested: bool,
    version_requested: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            depth_first: false,
            min_depth: 0,
            max_depth: usize::max_value(),
            sorted_output: false,
            help_requested: false,
            version_requested: false,
        }
    }
}

/// Trait that encapsulates various dependencies (output, clocks, etc.) that we
/// might want to fake out for unit tests.
pub trait Dependencies<'a> {
    fn get_output(&'a self) -> &'a RefCell<dyn Write>;
    fn now(&'a self) -> SystemTime;
}

/// Struct that holds the dependencies we use when run as the real executable.
pub struct StandardDependencies {
    output: Rc<RefCell<dyn Write>>,
    now: SystemTime,
}

impl StandardDependencies {
    pub fn new() -> StandardDependencies {
        StandardDependencies {
            output: Rc::new(RefCell::new(stdout())),
            now: SystemTime::now(),
        }
    }
}

impl Default for StandardDependencies {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Dependencies<'a> for StandardDependencies {
    fn get_output(&'a self) -> &'a RefCell<dyn Write> {
        self.output.as_ref()
    }

    fn now(&'a self) -> SystemTime {
        self.now
    }
}

/// The result of parsing the command-line arguments into useful forms.
struct ParsedInfo {
    matcher: Box<dyn self::matchers::Matcher>,
    paths: Vec<String>,
    config: Config,
}

/// Function to generate a `ParsedInfo` from the strings supplied on the command-line.
fn parse_args(args: &[&str]) -> Result<ParsedInfo, Box<dyn Error>> {
    let mut paths = vec![];
    let mut i = 0;
    let mut config = Config::default();

    while i < args.len()
        && (args[i] == "-" || !args[i].starts_with('-'))
        && args[i] != "!"
        && args[i] != "("
    {
        paths.push(args[i].to_string());
        i += 1;
    }
    if i == 0 {
        paths.push(".".to_string());
    }
    let matcher = matchers::build_top_level_matcher(&args[i..], &mut config)?;
    Ok(ParsedInfo {
        matcher,
        paths,
        config,
    })
}

fn process_dir<'a>(
    dir: &str,
    config: &Config,
    deps: &'a dyn Dependencies<'a>,
    matcher: &dyn matchers::Matcher,
) -> u64 {
    let mut found_count: u64 = 0;
    let mut walkdir = WalkDir::new(dir)
        .contents_first(config.depth_first)
        .max_depth(config.max_depth)
        .min_depth(config.min_depth);
    if config.sorted_output {
        walkdir = walkdir.sort_by(|a, b| a.file_name().cmp(b.file_name()));
    }

    // Slightly yucky loop handling here :-(. See docs for
    // WalkDirIterator::skip_current_dir for explanation.
    let mut it = walkdir.into_iter();
    loop {
        match it.next() {
            None => break,
            Some(Err(err)) => writeln!(&mut stderr(), "Error: {}: {}", dir, err).unwrap(),
            Some(Ok(entry)) => {
                let mut matcher_io = matchers::MatcherIO::new(deps);
                if matcher.matches(&entry, &mut matcher_io) {
                    found_count += 1;
                }
                if matcher_io.should_skip_current_dir() {
                    it.skip_current_dir();
                }
            }
        }
    }
    found_count
}

fn do_find<'a>(args: &[&str], deps: &'a dyn Dependencies<'a>) -> Result<u64, Box<dyn Error>> {
    let paths_and_matcher = parse_args(args)?;
    if paths_and_matcher.config.help_requested {
        print_help();
        return Ok(0);
    }
    if paths_and_matcher.config.version_requested {
        print_version();
        return Ok(0);
    }

    let mut found_count: u64 = 0;
    for path in paths_and_matcher.paths {
        found_count += process_dir(
            &path,
            &paths_and_matcher.config,
            deps,
            &*paths_and_matcher.matcher,
        );
    }
    Ok(found_count)
}

fn print_help() {
    println!(
        r"Usage: find [path...] [expression]

If no path is supplied then the current working directory is used by default.

Early alpha implementation. Currently the only expressions supported are
 -print
 -print0
 -printf
 -name case-sensitive_filename_pattern
 -iname case-insensitive_filename_pattern
 -type type_char
    currently type_char can only be f (for file) or d (for directory)
 -size [+-]N[bcwkMG]
 -delete
 -prune
 -not
 -a
 -o[r]
 ,
 ()
 -true
 -false
 -maxdepth N
 -mindepth N
 -d[epth]
 -ctime [+-]N
 -atime [+-]N
 -mtime [+-]N
 -perm [-/]{{octal|u=rwx,go=w}}
 -newer path_to_file
 -exec[dir] executable [args] [{{}}] [more args] ;
 -sorted
    a non-standard extension that sorts directory contents by name before
    processing them. Less efficient, but allows for deterministic output.
"
    );
}

fn print_version() {
    println!("find (Rust) {}", env!("CARGO_PKG_VERSION"));
}

/// Does all the work for find.
///
/// All main has to do is pass in the command-line args and exit the process
/// with the exit code. Note that the first string in args is expected to be
/// the name of the executable.
pub fn find_main<'a>(args: &[&str], deps: &'a dyn Dependencies<'a>) -> i32 {
    match do_find(&args[1..], deps) {
        Ok(_) => 0,
        Err(e) => {
            writeln!(&mut stderr(), "Error: {}", e).unwrap();
            1
        }
    }
}

#[cfg(test)]
mod tests {

    use std::cell::RefCell;
    use std::fs;
    use std::io::{Cursor, Read, Write};
    use std::time::{Duration, SystemTime};
    use std::vec::Vec;
    use tempfile::Builder;

    use crate::find::matchers::MatcherIO;

    use super::*;

    #[cfg(windows)]
    /// Windows-only bodge for converting between path separators.
    pub fn fix_up_slashes(path: &str) -> String {
        path.replace("/", "\\")
    }

    #[cfg(not(windows))]
    /// Do nothing equivalent of the above for non-windows systems.
    pub fn fix_up_slashes(path: &str) -> String {
        path.to_string()
    }

    /// A struct that implements Dependencies, but uses faked implementations,
    /// allowing us to check output, set the time returned by clocks etc.
    pub struct FakeDependencies {
        pub output: RefCell<Cursor<Vec<u8>>>,
        now: SystemTime,
    }

    impl<'a> FakeDependencies {
        pub fn new() -> FakeDependencies {
            FakeDependencies {
                output: RefCell::new(Cursor::new(Vec::<u8>::new())),
                now: SystemTime::now(),
            }
        }

        pub fn set_time(&mut self, new_time: SystemTime) {
            self.now = new_time;
        }

        pub fn new_matcher_io(&'a self) -> MatcherIO<'a> {
            MatcherIO::new(self)
        }

        pub fn get_output_as_string(&self) -> String {
            let mut cursor = self.output.borrow_mut();
            cursor.set_position(0);
            let mut contents = String::new();
            cursor.read_to_string(&mut contents).unwrap();
            contents
        }
    }

    impl<'a> Dependencies<'a> for FakeDependencies {
        fn get_output(&'a self) -> &'a RefCell<dyn Write> {
            &self.output
        }

        fn now(&'a self) -> SystemTime {
            self.now
        }
    }

    #[test]
    fn parse_args_handles_single_dash() {
        // Apparently "-" should be treated as a directory name.
        let parsed_info = super::parse_args(&["-"]).expect("parsing should succeed");
        assert_eq!(parsed_info.paths, ["-"]);
    }

    #[test]
    fn parse_args_bad_flag() {
        //
        let result = super::parse_args(&["-asdadsafsfsadcs"]);
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Unrecognized flag: '-asdadsafsfsadcs'");
        } else {
            panic!("parse_args should have returned an error");
        }
    }

    #[test]
    fn find_main_not_depth_first() {
        let deps = FakeDependencies::new();

        let rc = find_main(
            &["find", &fix_up_slashes("./test_data/simple"), "-sorted"],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/simple\n\
                 ./test_data/simple/abbbc\n\
                 ./test_data/simple/subdir\n\
                 ./test_data/simple/subdir/ABBBC\n"
            )
        );
    }

    #[test]
    fn find_main_depth_first() {
        let deps = FakeDependencies::new();

        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/simple"),
                "-sorted",
                "-depth",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/simple/abbbc\n\
                 ./test_data/simple/subdir/ABBBC\n\
                 ./test_data/simple/subdir\n\
                 ./test_data/simple\n"
            )
        );
    }

    #[test]
    fn find_maxdepth() {
        let deps = FakeDependencies::new();

        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-sorted",
                "-maxdepth",
                "2",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/depth\n\
                 ./test_data/depth/1\n\
                 ./test_data/depth/1/2\n\
                 ./test_data/depth/1/f1\n\
                 ./test_data/depth/f0\n"
            )
        );
    }

    #[test]
    fn find_maxdepth_depth_first() {
        let deps = FakeDependencies::new();

        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-sorted",
                "-maxdepth",
                "2",
                "-depth",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/depth/1/2\n\
                 ./test_data/depth/1/f1\n\
                 ./test_data/depth/1\n\
                 ./test_data/depth/f0\n\
                 ./test_data/depth\n"
            )
        );
    }

    #[test]
    fn find_prune() {
        let deps = FakeDependencies::new();

        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-sorted",
                "-print",
                ",",
                "-name",
                "1",
                "-prune",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/depth\n\
                 ./test_data/depth/1\n\
                 ./test_data/depth/f0\n"
            )
        );
    }

    #[test]
    fn find_zero_maxdepth() {
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-maxdepth",
                "0",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/depth\n")
        );
    }

    #[test]
    fn find_zero_maxdepth_depth_first() {
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-maxdepth",
                "0",
                "-depth",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/depth\n")
        );
    }

    #[test]
    fn find_mindepth() {
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-sorted",
                "-mindepth",
                "3",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/depth/1/2/3\n\
                 ./test_data/depth/1/2/3/f3\n\
                 ./test_data/depth/1/2/f2\n"
            )
        );
    }

    #[test]
    fn find_mindepth_depth_first() {
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-sorted",
                "-mindepth",
                "3",
                "-depth",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes(
                "./test_data/depth/1/2/3/f3\n\
                 ./test_data/depth/1/2/3\n\
                 ./test_data/depth/1/2/f2\n"
            )
        );
    }

    #[test]
    fn find_newer() {
        // create a temp directory and file that are newer than the static
        // files in the source tree.
        let new_dir = Builder::new().prefix("find_newer").tempdir().unwrap();

        let deps = FakeDependencies::new();

        let rc = find_main(
            &[
                "find",
                &new_dir.path().to_string_lossy(),
                "-newer",
                &fix_up_slashes("./test_data/simple/abbbc"),
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            (&new_dir).path().to_string_lossy().to_string() + "\n"
        );

        // now do it the other way around, and nothing should be output
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/simple/abbbc"),
                "-newer",
                &new_dir.path().to_string_lossy(),
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "");
    }

    #[test]
    fn find_mtime() {
        let meta = fs::metadata("./test_data/simple/subdir/ABBBC").unwrap();

        // metadata can return errors like StringError("creation time is not available on this platform currently")
        // so skip tests that won't pass due to shortcomings in std:;fs.
        if let Ok(file_time) = meta.modified() {
            file_time_helper(file_time, "-mtime");
        }
    }

    #[test]
    fn find_ctime() {
        let meta = fs::metadata("./test_data/simple/subdir/ABBBC").unwrap();

        // metadata can return errors like StringError("creation time is not available on this platform currently")
        // so skip tests that won't pass due to shortcomings in std:;fs.
        if let Ok(file_time) = meta.created() {
            file_time_helper(file_time, "-ctime");
        }
    }

    #[test]
    fn find_atime() {
        let meta = fs::metadata("./test_data/simple/subdir/ABBBC").unwrap();

        // metadata can return errors like StringError("creation time is not available on this platform currently")
        // so skip tests that won't pass due to shortcomings in std:;fs.
        if let Ok(file_time) = meta.accessed() {
            file_time_helper(file_time, "-atime");
        }
    }

    /// Helper function for the find_ctime/find_atime/find_mtime tests.
    fn file_time_helper(file_time: SystemTime, arg: &str) {
        // check file time matches a file that's old enough
        {
            let mut deps = FakeDependencies::new();
            deps.set_time(file_time);

            let rc = find_main(
                &[
                    "find",
                    &fix_up_slashes("./test_data/simple/subdir"),
                    "-type",
                    "f",
                    arg,
                    "0",
                ],
                &deps,
            );

            assert_eq!(rc, 0);
            assert_eq!(
                deps.get_output_as_string(),
                fix_up_slashes("./test_data/simple/subdir/ABBBC\n")
            );
        }

        // now Check file time doesn't match a file that's too new
        {
            let mut deps = FakeDependencies::new();
            deps.set_time(file_time - Duration::from_secs(1));

            let rc = find_main(
                &["find", "./test_data/simple/subdir", "-type", "f", arg, "0"],
                &deps,
            );

            assert_eq!(rc, 0);
            assert_eq!(deps.get_output_as_string(), "");
        }
    }

    #[test]
    fn find_size() {
        let deps = FakeDependencies::new();
        // only look at files because the "size" of a directory is a system (and filesystem)
        // dependent thing and we want these tests to be universal.
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/size"),
                "-type",
                "f",
                "-size",
                "1b",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/size/512bytes\n")
        );

        let deps = FakeDependencies::new();
        let rc = find_main(
            &["find", "./test_data/size", "-type", "f", "-size", "+1b"],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "");
    }
}
