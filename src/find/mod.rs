mod matchers;
use self::matchers::GivenPathInfo;

use std::error::Error;
use std::fs;
use std::path::Path;
use std::io::stderr;
use std::io::stdout;

use std::cell::RefCell;
use std::rc::Rc;
use std::io::Write;


pub struct Config {
    depth_first: bool,
    min_depth: u32,
    max_depth: u32,
}

impl Config {
    fn new() -> Config {
        Config {
            depth_first: false,
            min_depth: 0,
            max_depth: u32::max_value(),
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

/// Function that goes through a directory's contents, checking files, and calling
/// itself recursively for subdirectories.
fn process_dir<'a>(dir: &Path,
                   depth: u32,
                   config: &Config,
                   deps: &'a Dependencies<'a>,
                   matcher: &Box<matchers::Matcher>)
                   -> Result<i32, Box<Error>> {
    let mut found_count = 0;
    let this_dir = GivenPathInfo::new(dir);
    if !config.depth_first {
        let mut side_effects: matchers::SideEffectRefs<'a> = matchers::SideEffectRefs::new(deps);
        if depth >= config.min_depth && matcher.matches(&this_dir, &mut side_effects) {
            found_count += 1;
        }
        if side_effects.should_skip_current_dir {
            return Ok(found_count);
        }
    }

    match fs::read_dir(dir) {
        Ok(entry_results) => {
            for entry_result in entry_results {
                let entry = try!(entry_result);
                let path = entry.path();
                if path.is_dir() {
                    if depth < config.max_depth {
                        found_count += try!(process_dir(&path, depth + 1, config, deps, matcher));
                    }
                } else {
                    if depth + 1 >= config.min_depth && depth < config.max_depth {
                        let mut side_effects = matchers::SideEffectRefs::new(deps);
                        println!("about to call matcher");
                        if matcher.matches(&entry, &mut side_effects) {
                            found_count += 1;
                        }
                    }
                }
            }
        }
        Err(e) => {
            writeln!(&mut stderr(),
                     "Error: {}: {}",
                     dir.to_string_lossy(),
                     e.description())
                .unwrap();
        }
    }
    if config.depth_first {
        let mut side_effects = matchers::SideEffectRefs::new(deps);
        if depth >= config.min_depth && matcher.matches(&this_dir, &mut side_effects) {
            found_count += 1;
        }
    }
    Ok(found_count)
}


fn do_find<'a>(args: &[&str], deps: &'a Dependencies<'a>) -> Result<i32, Box<Error>> {
    let paths_and_matcher = try!(parse_args(args));
    let mut found_count = 0;
    for path in paths_and_matcher.paths {
        let dir = Path::new(&path);
        found_count += try!(process_dir(&dir,
                                        0,
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
mod test {


    use std::cell::RefCell;
    use std::vec::Vec;
    use std::io::Cursor;
    use std::io::Read;
    use std::io::Write;
    use super::Dependencies;

    /// A struct that implements Dependencies, but uses faked implementations,
    /// allowing us to check output, set the time returned by clocks etc.
    pub struct FakeDependencies {
        pub output: RefCell<Cursor<Vec<u8>>>,
    }

    impl<'a> FakeDependencies {
        pub fn new() -> FakeDependencies {
            FakeDependencies { output: RefCell::new(Cursor::new(Vec::<u8>::new())) }
        }

        pub fn new_side_effects(&'a self) -> super::matchers::SideEffectRefs<'a> {
            super::matchers::SideEffectRefs::new(self)
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


    // disabled until we can get deterministic directory contents ordering
    //    #[test]
    //    fn find_main_not_depth_first() {
    //        let output = new_output();
    //
    //        let rc = super::find_main(&["find", "./test_data/simple"], output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/simple\n\
    //                   ./test_data/simple/abbbc\n\
    //                   ./test_data/simple/subdir\n\
    //                   ./test_data/simple/subdir/ABBBC\n");
    //    }
    //
    //    #[test]
    //    fn find_main_depth_first() {
    //        let output = new_output();
    //
    //        let rc = super::find_main(&["find", "./test_data/simple", "-depth"], output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/simple/subdir/ABBBC\n\
    //                   ./test_data/simple/subdir\n\
    //                   ./test_data/simple/abbbc\n\
    //                   ./test_data/simple\n");
    //    }
    //
    //    #[test]
    //    fn find_maxdepth() {
    //        let output = new_output();
    //        let rc = super::find_main(&["find", "./test_data/depth", "-maxdepth", "2"],
    //                                  output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/depth\n\
    //                   ./test_data/depth/f0\n\
    //                   ./test_data/depth/1\n\
    //                   ./test_data/depth/1/f1\n\
    //                   ./test_data/depth/1/2\n");
    //    }
    //
    //    #[test]
    //    fn find_maxdepth_depth_first() {
    //        let output = new_output();
    //        let rc = super::find_main(&["find", "./test_data/depth", "-maxdepth", "2", "-depth"],
    //                                  output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/depth/1/2\n\
    //                   ./test_data/depth/1/f1\n\
    //                   ./test_data/depth/1\n\
    //                   ./test_data/depth/f0\n\
    //                   ./test_data/depth\n");
    //    }
    //
    //    #[test]
    //    fn find_prune() {
    //        let output = new_output();
    //        let rc =
    //            super::find_main(&["find", "./test_data/depth", "-print", ",", "-name", "1", "-prune"],
    //                             output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/depth/1\n./test_data/depth/f0\n./test_data/depth\n");
    //    }

    #[test]
    fn find_zero_maxdepth() {
        let deps = FakeDependencies::new();
        let rc = super::find_main(&["find", "./test_data/depth", "-maxdepth", "0"], &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "./test_data/depth\n");
    }

    #[test]
    fn find_zero_maxdepth_depth_first() {
        let deps = FakeDependencies::new();
        let rc = super::find_main(&["find", "./test_data/depth", "-maxdepth", "0", "-depth"],
                                  &deps);

        assert_eq!(rc, 0);
        assert_eq!(deps.get_output_as_string(), "./test_data/depth\n");
    }

    // disabled until we can get deterministic directory contents ordering
    //    #[test]
    //    fn find_mindepth() {
    //        let output = new_output();
    //        let rc = super::find_main(&["find", "./test_data/depth", "-mindepth", "3"],
    //                                  output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/depth/1/2/f2\n\
    //                   ./test_data/depth/1/2/3\n\
    //                   ./test_data/depth/1/2/3/f3\n");
    //    }
    //
    //    #[test]
    //    fn find_mindepth_depth_first() {
    //        let output = new_output();
    //        let rc = super::find_main(&["find", "./test_data/depth", "-mindepth", "3", "-depth"],
    //                                  output.clone());
    //
    //        assert_eq!(rc, 0);
    //        assert_eq!(get_output_as_string(&output),
    //                   "./test_data/depth/1/2/3/f3\n\
    //                   ./test_data/depth/1/2/3\n\
    //                   ./test_data/depth/1/2/f2\n");
    //    }

}
