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
    same_file_system: bool,
    depth_first: bool,
    min_depth: usize,
    max_depth: usize,
    sorted_output: bool,
    help_requested: bool,
    version_requested: bool,
    no_leaf_dirs: bool,
    follow: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            same_file_system: false,
            depth_first: false,
            min_depth: 0,
            max_depth: usize::MAX,
            sorted_output: false,
            help_requested: false,
            version_requested: false,
            // Directory information and traversal are done by walkdir,
            // and this configuration field will exist as
            // a compatibility item for GNU findutils.
            no_leaf_dirs: false,
            follow: false,
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
    #[must_use]
    pub fn new() -> Self {
        Self {
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

    while i < args.len() {
        match args[i] {
            "-O0" | "-O1" | "-O2" | "-O3" => {
                // GNU find optimization level flag (ignored)
            }
            "-P" => {
                // Never follow symlinks (the default)
            }
            "--" => {
                // End of flags
                i += 1;
                break;
            }
            _ => break,
        }

        i += 1;
    }

    let paths_start = i;
    while i < args.len()
        && (args[i] == "-" || !args[i].starts_with('-'))
        && args[i] != "!"
        && args[i] != "("
    {
        paths.push(args[i].to_string());
        i += 1;
    }
    if i == paths_start {
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
    quit: &mut bool,
) -> u64 {
    let mut found_count: u64 = 0;
    let mut walkdir = WalkDir::new(dir)
        .contents_first(config.depth_first)
        .max_depth(config.max_depth)
        .min_depth(config.min_depth)
        .same_file_system(config.same_file_system);
    if config.sorted_output {
        walkdir = walkdir.sort_by(|a, b| a.file_name().cmp(b.file_name()));
    }

    // Slightly yucky loop handling here :-(. See docs for
    // WalkDirIterator::skip_current_dir for explanation.
    let mut it = walkdir.into_iter();
    while let Some(result) = it.next() {
        match result {
            Err(err) => {
                uucore::error::set_exit_code(1);
                writeln!(&mut stderr(), "Error: {dir}: {err}").unwrap()
            }
            Ok(entry) => {
                let mut matcher_io = matchers::MatcherIO::new(deps);

                if matcher.matches(&entry, &mut matcher_io) {
                    found_count += 1;
                }
                if matcher_io.should_quit() {
                    *quit = true;
                    break;
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
    let mut quit = false;
    for path in paths_and_matcher.paths {
        found_count += process_dir(
            &path,
            &paths_and_matcher.config,
            deps,
            &*paths_and_matcher.matcher,
            &mut quit,
        );
        if quit {
            break;
        }
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
 -lname case-sensitive_filename_pattern
 -iname case-insensitive_filename_pattern
 -ilname case-insensitive_filename_pattern
 -regextype type
 -regex pattern
 -iregex pattern
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
 -xdev
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
        Ok(_) => uucore::error::get_exit_code(),
        Err(e) => {
            writeln!(&mut stderr(), "Error: {e}").unwrap();
            1
        }
    }
}

#[cfg(test)]
mod tests {

    use std::fs;
    use std::io::{Cursor, ErrorKind, Read};
    use std::time::Duration;
    use tempfile::Builder;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(windows)]
    use std::os::windows::fs::symlink_file;

    use crate::find::matchers::time::ChangeTime;
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
        pub fn new() -> Self {
            Self {
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

    fn create_file_link() {
        #[cfg(unix)]
        if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {e:?}"
            );
        }
        #[cfg(windows)]
        if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {:?}",
                e
            );
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
    fn parse_optimize_flag() {
        let parsed_info =
            super::parse_args(&["-O0", ".", "-print"]).expect("parsing should succeed");
        assert_eq!(parsed_info.paths, ["."]);
    }

    #[test]
    fn parse_p_flag() {
        super::parse_args(&["-P"]).expect("parsing should succeed");
    }

    #[test]
    fn parse_flag_then_double_dash() {
        super::parse_args(&["-P", "--"]).expect("parsing should succeed");
    }

    #[test]
    fn parse_double_dash_then_flag() {
        super::parse_args(&["--", "-P"])
            .err()
            .expect("parsing should fail");
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
            new_dir.path().to_string_lossy().to_string() + "\n"
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
        // so skip tests that won't pass due to shortcomings in std::fs.
        if let Ok(file_time) = meta.modified() {
            file_time_helper(file_time, "-mtime");
        }
    }

    #[test]
    fn find_ctime() {
        let meta = fs::metadata("./test_data/simple/subdir/ABBBC").unwrap();

        // metadata can return errors like StringError("creation time is not available on this platform currently")
        // so skip tests that won't pass due to shortcomings in std::fs.
        if let Ok(file_time) = meta.changed() {
            file_time_helper(file_time, "-ctime");
        }
    }

    #[test]
    fn find_atime() {
        let meta = fs::metadata("./test_data/simple/subdir/ABBBC").unwrap();

        // metadata can return errors like StringError("creation time is not available on this platform currently")
        // so skip tests that won't pass due to shortcomings in std::fs.
        if let Ok(file_time) = meta.accessed() {
            file_time_helper(file_time, "-atime");
        }
    }

    /// Helper function for the `find_ctime/find_atime/find_mtime` tests.
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

    // Because the time when files exist locally is different
    // from the time when Github Actions pulls them,
    // it is difficult to write tests that limit a certain time period.
    //
    // For example, a Github Action may pull files from a new git commit within a few minutes,
    // causing the file time to be refreshed to the pull time.
    // and The files on the local branch may be several days old.
    //
    // So this test may not be too accurate and can only ensure that
    // the function can be correctly identified.
    #[test]
    fn find_amin_cmin_mmin() {
        let args = ["-amin", "-cmin", "-mmin"];
        let times = ["-60", "-120", "-240", "+60", "+120", "+240"];

        for arg in args {
            for time in times {
                let deps = FakeDependencies::new();
                let rc = find_main(&["find", "./test_data/simple/subdir", arg, time], &deps);

                assert_eq!(rc, 0);
            }
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

    #[test]
    fn find_name_links() {
        create_file_link();

        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/links"),
                "-name",
                "abbbc",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/links/abbbc\n")
        );
    }

    #[test]
    fn find_lname_links() {
        create_file_link();

        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/links"),
                "-lname",
                "abbbc",
                "-sorted",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/links/link-f\n")
        );
    }

    #[test]
    fn find_ilname_links() {
        create_file_link();

        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/links"),
                "-ilname",
                "abBbc",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/links/link-f\n")
        );
    }

    #[test]
    fn find_print_then_quit() {
        let deps = FakeDependencies::new();

        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/simple"),
                &fix_up_slashes("./test_data/simple"),
                "-print",
                "-quit",
            ],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/simple\n"),
        );
    }

    #[test]
    fn test_find_newer_xy_all_args() {
        // 1. The t parameter is not allowed at the X position.
        // 2. Current Linux filesystem do not support Birthed Time queries,
        //    so the B parameter will be excluded in linux.
        #[cfg(target_os = "linux")]
        let x_options = ["a", "c", "m"];
        #[cfg(not(target_os = "linux"))]
        let x_options = ["a", "B", "c", "m"];
        #[cfg(target_os = "linux")]
        let y_options = ["a", "c", "m"];
        #[cfg(not(target_os = "linux"))]
        let y_options = ["a", "B", "c", "m"];

        for &x in x_options.iter() {
            for &y in &y_options {
                let arg = &format!("-newer{x}{y}").to_string();
                let deps = FakeDependencies::new();
                let rc = find_main(
                    &[
                        "find",
                        "./test_data/simple/subdir",
                        arg,
                        "./test_data/simple/subdir/ABBBC",
                    ],
                    &deps,
                );

                assert_eq!(rc, 0);

                let arg = &format!("-follow -newer{x}{y}").to_string();
                let deps = FakeDependencies::new();
                let rc = find_main(
                    &[
                        "find",
                        "./test_data/simple/subdir",
                        arg,
                        "./test_data/simple/subdir/ABBBC",
                    ],
                    &deps,
                );

                assert_eq!(rc, 0);
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_find_newer_xy_have_not_birthed_time_filesystem() {
        let y_options = ["a", "c", "m"];
        for &y in &y_options {
            let arg = &format!("-newerB{y}").to_string();
            let deps = FakeDependencies::new();
            let rc = find_main(
                &[
                    "find",
                    "./test_data/simple/subdir",
                    arg,
                    "./test_data/simple/subdir/ABBBC",
                ],
                &deps,
            );

            assert_eq!(rc, 1);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_find_newer_xy_before_changed_time() {
        // normal - before the changed time
        #[cfg(target_os = "linux")]
        let args = ["-newerat", "-newerct", "-newermt"];
        #[cfg(not(target_os = "linux"))]
        let args = ["-newerat", "-newerBt", "-newerct", "-newermt"];
        let times = ["jan 01, 2000", "jan 01, 2000 00:00:00"];

        for arg in args {
            for time in times {
                let deps = FakeDependencies::new();
                let rc = find_main(&["find", "./test_data/simple/subdir", arg, time], &deps);

                assert_eq!(rc, 0);
                assert!(deps
                    .get_output_as_string()
                    .contains("./test_data/simple/subdir"));
                assert!(deps.get_output_as_string().contains("ABBBC"));
            }
        }
    }

    #[test]
    fn test_find_newer_xy_after_changed_time() {
        // normal - after the changed time
        #[cfg(target_os = "linux")]
        let args = ["-newerat", "-newerct", "-newermt"];
        #[cfg(not(target_os = "linux"))]
        let args = ["-newerat", "-newerBt", "-newerct", "-newermt"];
        let times = ["jan 01, 2037", "jan 01, 2037 00:00:00"];

        for arg in args {
            for time in times {
                let deps = FakeDependencies::new();
                let rc = find_main(&["find", "./test_data/simple/subdir", arg, time], &deps);

                assert_eq!(rc, 0);
                assert_eq!(deps.get_output_as_string(), "");
            }
        }
    }

    #[test]
    fn test_find_newer_xy_empty_time_parameter() {
        // When an empty time parameter is passed in,
        // the program will use 00:00 of the current day as the default time.
        // Therefore, the files checkout of the git repository while
        // this test was running are likely to be newer than the default time.
        #[cfg(target_os = "linux")]
        let args = ["-newerat", "-newerct", "-newermt"];
        #[cfg(not(target_os = "linux"))]
        let args = ["-newerat", "-newerBt", "-newerct", "-newermt"];
        let time = "";

        for &arg in &args {
            let deps = FakeDependencies::new();
            let rc = find_main(&["find", "./test_data/simple/subdir", arg, time], &deps);

            assert_eq!(rc, 0);
            // Output comparison has been temporarily removed to account for the possibility that
            // migration out of the repository started before 00:00 and testing was completed after 00:00.
        }
    }

    #[test]
    fn test_find_newer_xy_error_time() {
        // Catch a parsing error.
        #[cfg(target_os = "linux")]
        let args = ["-newerat", "-newerct", "-newermt"];
        #[cfg(not(target_os = "linux"))]
        let args = ["-newerat", "-newerBt", "-newerct", "-newermt"];
        let time = "2037, jan 01";

        for &arg in &args {
            let deps = FakeDependencies::new();
            let rc = find_main(&["find", "./test_data/simple/subdir", arg, time], &deps);

            assert_eq!(rc, 1);
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_no_permission_file_error() {
        use std::{path::Path, process::Command};

        let path = Path::new("./test_data/no_permission");
        let _result = fs::create_dir(path);
        // Generate files without permissions.
        // std::fs cannot change file permissions to 000 in normal user state,
        // so use chmod via Command to change permissions.
        let _output = Command::new("chmod")
            .arg("-rwx")
            .arg("./test_data/no_permission")
            .output()
            .expect("cannot set file permission");

        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/no_permission"], &deps);

        assert_eq!(rc, 1);

        // Reset the exit code global variable in case we run another test after this one
        // See https://github.com/uutils/coreutils/issues/5777
        uucore::error::set_exit_code(0);

        if path.exists() {
            let _result = fs::create_dir(path);
            // Remove the unreadable and writable status of the file to avoid affecting other tests.
            let _output = Command::new("chmod")
                .arg("+rwx")
                .arg("./test_data/no_permission")
                .output()
                .expect("cannot set file permission");
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_user_predicate() {
        use std::{os::unix::fs::MetadataExt, path::Path};

        use nix::unistd::{Uid, User};

        let path = Path::new("./test_data/simple/subdir");
        let uid = path.metadata().unwrap().uid();
        let user = User::from_uid(Uid::from_raw(uid)).unwrap().unwrap().name;

        let deps = FakeDependencies::new();
        let rc = find_main(
            &["find", "./test_data/simple/subdir", "-user", &user],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            "./test_data/simple/subdir\n./test_data/simple/subdir/ABBBC\n"
        );

        // test uid
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                "./test_data/simple/subdir",
                "-uid",
                &uid.to_string(),
            ],
            &deps,
        );
        assert_eq!(rc, 0);

        // test empty uid
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-uid", ""], &deps);
        assert_eq!(rc, 1);

        // test not a number
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-uid", "a"], &deps);
        assert_eq!(rc, 1);

        // test empty user name
        ["-user", "-nouser"].iter().for_each(|&arg| {
            let deps = FakeDependencies::new();
            let rc = find_main(&["find", "./test_data/simple/subdir", arg, ""], &deps);

            assert_eq!(rc, 1);

            let deps = FakeDependencies::new();
            let rc = find_main(&["find", "./test_data/simple/subdir", arg, " "], &deps);

            assert_eq!(rc, 1);
        });
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_nouser_predicate() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-nouser"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_group_predicate() {
        use std::{os::unix::fs::MetadataExt, path::Path};

        use nix::unistd::{Gid, Group};

        let path = Path::new("./test_data/simple/subdir");
        let gid = path.metadata().unwrap().gid();
        let group = Group::from_gid(Gid::from_raw(gid)).unwrap().unwrap().name;

        let deps = FakeDependencies::new();
        let rc = find_main(
            &["find", "./test_data/simple/subdir", "-group", &group],
            &deps,
        );

        assert_eq!(rc, 0);
        assert_eq!(
            deps.get_output_as_string(),
            "./test_data/simple/subdir\n./test_data/simple/subdir/ABBBC\n"
        );

        // test gid
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                "./test_data/simple/subdir",
                "-gid",
                gid.to_string().as_str(),
            ],
            &deps,
        );
        assert_eq!(rc, 0);

        // test empty gid
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-gid", ""], &deps);
        assert_eq!(rc, 1);

        // test not a number
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-gid", "a"], &deps);
        assert_eq!(rc, 1);

        // test empty user name and group name
        ["-group", "-nogroup"].iter().for_each(|&arg| {
            let deps = FakeDependencies::new();
            let rc = find_main(&["find", "./test_data/simple/subdir", arg, ""], &deps);

            assert_eq!(rc, 1);

            let deps = FakeDependencies::new();
            let rc = find_main(&["find", "./test_data/simple/subdir", arg, " "], &deps);

            assert_eq!(rc, 1);
        });
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_nogroup_predicate() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-nogroup"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "");
    }

    #[test]
    #[cfg(unix)]
    fn test_fs_matcher() {
        use crate::find::tests::FakeDependencies;
        use matchers::fs::get_file_system_type;
        use std::cell::RefCell;
        use std::path::Path;

        let path = Path::new("./test_data/simple/subdir");
        let empty_cache = RefCell::new(None);
        let target_fs_type = get_file_system_type(path, &empty_cache).unwrap();

        // should match fs type
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                "./test_data/simple/subdir",
                "-fstype",
                &target_fs_type,
            ],
            &deps,
        );

        assert_eq!(rc, 0);
    }

    #[test]
    #[cfg(unix)]
    fn test_noleaf() {
        use crate::find::tests::FakeDependencies;

        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple/subdir", "-noleaf"], &deps);

        assert_eq!(rc, 0);
    }

    #[test]
    fn find_maxdepth_and() {
        let deps = FakeDependencies::new();
        let rc = find_main(
            &[
                "find",
                &fix_up_slashes("./test_data/depth"),
                "-maxdepth",
                "0",
                "-a",
                "-print",
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
    fn test_follow() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/simple", "-follow"], &deps);
        assert_eq!(rc, 0);
    }
}
