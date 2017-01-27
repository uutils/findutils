mod matchers;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::io::stderr;

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

pub struct Config {
    depth_first: bool,
}

impl Config {
    fn new() -> Config {
        Config { depth_first: false }
    }
}

/// The result of parsing the command-line arguments into useful forms.
struct ParsedInfo {
    matcher: Box<self::matchers::Matcher>,
    paths: Vec<String>,
    config: Config,
}


fn parse_args(args: &[&str], output: Rc<RefCell<Write>>) -> Result<ParsedInfo, Box<Error>> {
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
    let matcher = try!(matchers::build_top_level_matcher(&args[i..], &mut config, output));
    Ok(ParsedInfo {
        matcher: matcher,
        paths: paths,
        config: config,
    })
}

fn process_dir(dir: &Path,
               config: &Config,
               matcher: &Box<matchers::Matcher>)
               -> Result<i32, Box<Error>> {
    let mut found_count = 0;
    match fs::read_dir(dir) {
        Ok(entry_results) => {
            let mut file_entries = vec![];
            let mut dir_entries = vec![];
            for entry_result in entry_results {
                let entry = try!(entry_result);
                let path = entry.path();
                if path.is_dir() {
                    dir_entries.push(entry);
                } else {
                    file_entries.push(entry);
                }
            }
            let entry_lists = if config.depth_first {
                vec![dir_entries, file_entries]
            } else {
                vec![file_entries, dir_entries]
            };
            for entry_list in entry_lists {
                for entry in entry_list {
                    if !config.depth_first {
                        if matcher.matches(&entry) {
                            found_count += 1;
                        }
                    }
                    let path = entry.path();
                    if path.is_dir() {
                        found_count += try!(process_dir(&path, config, matcher));
                    }
                    if config.depth_first {
                        if matcher.matches(&entry) {
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
    Ok(found_count)
}


fn do_find(args: &[&str], output: Rc<RefCell<Write>>) -> Result<i32, Box<Error>> {

    let paths_and_matcher = try!(parse_args(args, output));
    let mut found_count = 0;
    for path in paths_and_matcher.paths {
        let dir = Path::new(&path);
        found_count +=
            try!(process_dir(&dir, &paths_and_matcher.config, &paths_and_matcher.matcher));
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
pub fn find_main(args: &[&str], output: Rc<RefCell<Write>>) -> i32 {

    for arg in args {
        match arg.as_ref() {
            "-help" | "--help" => {
                print_help();
                return 0;
            }
            _ => (),
        }
    }
    match do_find(&args[1..], output) {
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
    use std::rc::Rc;
    use std::io::Read;


    pub fn new_output() -> Rc<RefCell<Cursor<Vec<u8>>>> {
        Rc::new(RefCell::new(Cursor::new(Vec::<u8>::new())))
    }

    pub fn get_output_as_string(output: &RefCell<Cursor<Vec<u8>>>) -> String {
        let mut cursor = output.borrow_mut();
        cursor.set_position(0);
        let mut contents = String::new();
        cursor.read_to_string(&mut contents).unwrap();
        contents
    }

    #[test]
    fn find_main_not_depth_first() {
        let output = new_output();

        let rc = super::find_main(&["find", "./test_data/simple"], output.clone());

        assert_eq!(rc, 0);
        assert_eq!(get_output_as_string(&output),
                   "./test_data/simple/abbbc\n\
                   ./test_data/simple/subdir\n\
                   ./test_data/simple/subdir/ABBBC\n");
    }

    #[test]
    fn find_main_depth_first() {
        let output = new_output();

        let rc = super::find_main(&["find", "./test_data/simple", "-depth"], output.clone());

        assert_eq!(rc, 0);
        assert_eq!(get_output_as_string(&output),
                   "./test_data/simple/subdir/ABBBC\n\
                   ./test_data/simple/subdir\n\
                   ./test_data/simple/abbbc\n");
    }
}
