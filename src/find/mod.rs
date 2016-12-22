mod matchers;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::io::stderr;
use std::io::Write;

struct PathsAndMatcher {
    matcher: Box<self::matchers::Matcher>,
    paths: Vec<String>,
}

fn parse_args(args: &[String]) -> Result<PathsAndMatcher, Box<Error>> {
    let mut paths = vec![];
    let mut i = 0;

    while i < args.len() && !args[i].starts_with('-') {
        paths.push(args[i].clone());
        i += 1;
    }
    if i == 0 {
        paths.push(".".to_string());
    }
    let matcher = try!(matchers::build_top_level_matcher(&args[i..]));
    Ok(PathsAndMatcher {
        matcher: matcher,
        paths: paths,
    })
}

fn process_dir(dir: &Path, matcher: &Box<matchers::Matcher>) -> Result<i32, Box<Error>> {
    let mut found_count = 0;
    match fs::read_dir(dir) {
        Ok(entry_results) => {
            for entry_result in entry_results {
                let entry = try!(entry_result);
                let path = entry.path();
                if matcher.matches(&entry) {
                    found_count += 1;
                }
                if path.is_dir() {
                    try!(process_dir(&path, matcher));
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


fn do_find(args: &[String]) -> Result<i32, Box<Error>> {

    let paths_and_matcher = try!(parse_args(args));
    let mut found_count = 0;
    for path in paths_and_matcher.paths {
        let dir = Path::new(&path);
        found_count += try!(process_dir(&dir, &paths_and_matcher.matcher));
    }
    Ok(found_count)
}

fn print_help() {
    println!("Usage: find [path...] [expression]

If no path is supplied then the current \
              working directory is used by default.

Early alpha implementation. Currently the \
              only expressions supported are
 -print
 -name case-sensitive_filename_pattern
 \
              -iname case-insensitive_filename_pattern
");
}

/// Does all the work for find.
///
/// All main has to do is pass in the command-line args and exit the process
/// with the exit code. Note that the first string in args is expected to be
/// the name of the executable.
pub fn find_main(args: &Vec<String>) -> i32 {

    for arg in args {
        match arg.as_ref() {
            "-help" | "--help" => {
                print_help();
                return 0;
            }
            _ => (),
        }
    }
    match do_find(&args[1..]) {
        Ok(_) => 0,
        Err(e) => {
            writeln!(&mut stderr(), "Error: {}", e).unwrap();
            1
        }
    }
}
