mod matchers;

use std::cell::RefCell;
use std::error::Error;
use std::io::{Write, stderr, stdout};
use std::rc::Rc;
use walkdir::WalkDir;
use walkdir::WalkDirIterator;

pub struct Config {
    depth_first: bool,
    min_depth: usize,
    max_depth: usize,
    sorted_output: bool,
}

impl Config {
    fn new() -> Config {
        Config {
            depth_first: false,
            min_depth: 0,
            max_depth: usize::max_value(),
            sorted_output: false,
        }
    }
}

/// Trait that encapsulates various dependencies (output, clocks, etc.) that we
/// might want to fake out for unit tests.
pub trait Dependencies<'a> {
    fn get_output(&'a self) -> &'a RefCell<Write>;
}

/// Struct that holds the dependencies we use when run as the real executable.
pub struct StandardDependencies {
    output: Rc<RefCell<Write>>,
}

impl StandardDependencies {
    pub fn new() -> StandardDependencies {
        StandardDependencies { output: Rc::new(RefCell::new(stdout())) }
    }
}

impl<'a> Dependencies<'a> for StandardDependencies {
    fn get_output(&'a self) -> &'a RefCell<Write> {
        self.output.as_ref()
    }
}

/// The result of parsing the command-line arguments into useful forms.
struct ParsedInfo {
    matcher: Box<self::matchers::Matcher>,
    paths: Vec<String>,
    config: Config,
}

/// Function to generate a ParsedInfoi from the strings supplied on the command-line.
fn parse_args(args: &[&str]) -> Result<ParsedInfo, Box<Error>> {
    let mut paths = vec![];
    let mut i = 0;
    let mut config = Config::new();

    while i < args.len() && !args[i].starts_with('-') && args[i] != "!" && args[i] != "(" {
        paths.push(args[i].to_string());
        i += 1;
    }
    if i == 0 {
        paths.push(".".to_string());
    }
    let matcher = try!(matchers::build_top_level_matcher(&args[i..], &mut config));
    Ok(ParsedInfo {
        matcher: matcher,
        paths: paths,
        config: config,
    })
}

fn process_dir<'a>(dir: &str,
                   config: &Config,
                   deps: &'a Dependencies<'a>,
                   matcher: &Box<matchers::Matcher>)
                   -> Result<i32, Box<Error>> {

    let mut found_count = 0;
    let mut walkdir = WalkDir::new(dir)
        .contents_first(config.depth_first)
        .max_depth(config.max_depth)
        .min_depth(config.min_depth);
    if config.sorted_output {
        walkdir = walkdir.sort_by(|a, b| a.cmp(b));
    }

    // Slighly yucky loop handling here :-(. See docs for
    // WalkDirIterator::skip_current_dir for explanation.
    let mut it = walkdir.into_iter();
    loop {
        match it.next() {
            None => break,
            Some(Err(err)) => {
                writeln!(&mut stderr(), "Error: {}: {}", dir, err.description()).unwrap()
            }
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
    Ok(found_count)
}


fn do_find<'a>(args: &[&str], deps: &'a Dependencies<'a>) -> Result<i32, Box<Error>> {
    let paths_and_matcher = try!(parse_args(args));
    let mut found_count = 0;
    for path in paths_and_matcher.paths {
        found_count += try!(process_dir(&path,
                                        &paths_and_matcher.config,
                                        deps,
                                        &paths_and_matcher.matcher));
    }
    Ok(found_count)
}

fn print_help() {
    println!("Usage: find [path...] [expression]

If no path is supplied then the current working directory is used by default.

Early alpha implementation. Currently the only expressions supported are
 -print
 -name case-sensitive_filename_pattern
 -iname case-insensitive_filename_pattern
 -type type_char
    currently type_char can only be f (for file) or d (for directory) 
");
}

/// Does all the work for find.
///
/// All main has to do is pass in the command-line args and exit the process
/// with the exit code. Note that the first string in args is expected to be
/// the name of the executable.
pub fn find_main<'a>(args: &[&str], deps: &'a Dependencies<'a>) -> i32 {

    for arg in args {
        match arg.as_ref() {
            "-help" | "--help" => {
                print_help();
                return 0;
            }
            _ => (),
        }
    }
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
    use std::io::{Cursor, Read, Write};
    use std::vec::Vec;
    use find::matchers::MatcherIO;

    use super::*;

    /// A struct that implements Dependencies, but uses faked implementations,
    /// allowing us to check output, set the time returned by clocks etc.
    pub struct FakeDependencies {
        pub output: RefCell<Cursor<Vec<u8>>>,
    }

    impl<'a> FakeDependencies {
        pub fn new() -> FakeDependencies {
            FakeDependencies { output: RefCell::new(Cursor::new(Vec::<u8>::new())) }
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
        fn get_output(&'a self) -> &'a RefCell<Write> {
            &self.output
        }
    }


    #[test]
    fn find_main_not_depth_first() {
        let deps = FakeDependencies::new();


        let rc = find_main(&["find", "./test_data/simple", "-sorted"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/simple\n\
                   ./test_data/simple/abbbc\n\
                   ./test_data/simple/subdir\n\
                   ./test_data/simple/subdir/ABBBC\n");
    }

    #[test]
    fn find_main_depth_first() {
        let deps = FakeDependencies::new();


        let rc = find_main(&["find", "./test_data/simple", "-depth"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/simple/subdir/ABBBC\n\
                   ./test_data/simple/subdir\n\
                   ./test_data/simple/abbbc\n\
                   ./test_data/simple\n");
    }

    #[test]
    fn find_maxdepth() {
        let deps = FakeDependencies::new();

        let rc = find_main(&["find", "./test_data/depth", "-sorted", "-maxdepth", "2"],
                           &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/depth\n\
                   ./test_data/depth/1\n\
                   ./test_data/depth/1/2\n\
                   ./test_data/depth/1/f1\n\
                   ./test_data/depth/f0\n");
    }

    #[test]
    fn find_maxdepth_depth_first() {
        let deps = FakeDependencies::new();

        let rc = find_main(&["find", "./test_data/depth", "-maxdepth", "2", "-depth"],
                           &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/depth/1/2\n\
                   ./test_data/depth/1/f1\n\
                   ./test_data/depth/1\n\
                   ./test_data/depth/f0\n\
                   ./test_data/depth\n");
    }

    #[test]
    fn find_prune() {
        let deps = FakeDependencies::new();

        let rc = find_main(&["find",
                             "./test_data/depth",
                             "-sorted",
                             "-print",
                             ",",
                             "-name",
                             "1",
                             "-prune"],
                           &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/depth\n\
                   ./test_data/depth/1\n\
                   ./test_data/depth/f0\n");
    }

    #[test]
    fn find_zero_maxdepth() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/depth", "-maxdepth", "0"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "./test_data/depth\n");
    }

    #[test]
    fn find_zero_maxdepth_depth_first() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/depth", "-maxdepth", "0", "-depth"],
                           &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "./test_data/depth\n");
    }

    #[test]
    fn find_mindepth() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/depth", "-mindepth", "3"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/depth/1/2/3\n\
                   ./test_data/depth/1/2/3/f3\n\
                   ./test_data/depth/1/2/f2\n");
    }

    #[test]
    fn find_mindepth_depth_first() {
        let deps = FakeDependencies::new();
        let rc = find_main(&["find", "./test_data/depth", "-mindepth", "3", "-depth"],
                           &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(),
                   "./test_data/depth/1/2/3/f3\n\
                   ./test_data/depth/1/2/3\n\
                   ./test_data/depth/1/2/f2\n");
    }

}
