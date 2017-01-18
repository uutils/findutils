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
pub fn build_top_level_matcher(args: &[&str],
                               output: Rc<RefCell<Write>>)
                               -> Result<Box<Matcher>, Box<Error>> {
    let (_, top_level_matcher) = try!(build_matcher_tree(args, output.clone(), 0, false));

    // if the matcher doesn't have any side-effects, then we default to printing
    if !top_level_matcher.has_side_effects() {
        let mut new_and_matcher = logical_matchers::AndMatcher::new();
        new_and_matcher.push(top_level_matcher);
        new_and_matcher.push(Box::new(printer::Printer::new(output)));
        return Ok(Box::new(new_and_matcher));
    }
    Ok(top_level_matcher)
}

/// Helper function for build_matcher_tree
fn are_more_expressions(args: &[&str], index: usize) -> bool {
    (index < args.len() - 1) && args[index + 1] != ")"
}


/// The main "translate command-line args into a matcher" function. Will call
/// itself recursively if it encounters an opening bracket. A successful return
/// consits of a tuple containing the new index into the args array to use (if
/// called recursively) and the resulting matcher.
fn build_matcher_tree(args: &[&str],
                      output: Rc<RefCell<Write>>,
                      arg_index: usize,
                      expecting_bracket: bool)
                      -> Result<(usize, Box<Matcher>), Box<Error>> {
    let mut top_level_matcher = logical_matchers::ListMatcher::new();

    // can't use getopts for a variety or reasons:
    // order of arguments is important
    // arguments can start with + as well as -
    // multiple-character flags don't start with a double dash
    let mut i = arg_index;
    let mut invert_next_matcher = false;
    while i < args.len() {
        let possible_submatcher = match args[i] {
            "-print" => Some(Box::new(printer::Printer::new(output.clone())) as Box<Matcher>),
            "-true" => Some(Box::new(logical_matchers::TrueMatcher {}) as Box<Matcher>),
            "-false" => Some(Box::new(logical_matchers::FalseMatcher {}) as Box<Matcher>),
            "-name" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(Box::new(try!(name_matcher::NameMatcher::new(args[i]
                    .as_ref()))) as Box<Matcher>)
            }
            "-iname" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(Box::new(try!(caseless_name_matcher::CaselessNameMatcher::new(args[i]))) as Box<Matcher>)
            }
            "-type" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(Box::new(try!(type_matcher::TypeMatcher::new(args[i]))) as Box<Matcher>)
            }
            "-not" | "!" => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!("expected an expression after {}", args[i])));
                }
                invert_next_matcher = true;
                None
            }
            "-or" | "-o" => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!("expected an expression after {}", args[i])));
                }
                try!(top_level_matcher.new_ored_criterion(args[i]));
                None
            }
            "," => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!("expected an expression after {}", args[i])));
                }
                try!(top_level_matcher.new_list_entry());
                None
            }
            "(" => {
                let (new_arg_index, sub_matcher) =
                    try!(build_matcher_tree(args, output.clone(), i + 1, true));
                i = new_arg_index;
                Some(sub_matcher)
            }
            ")" => {
                if !expecting_bracket {
                    return Err(From::from("you have too many ')'"));
                }
                return Ok((i, Box::new(top_level_matcher)));
            }

            _ => return Err(From::from(format!("Unrecognized flag: '{}'", args[i]))),
        };
        if let Some(submatcher) = possible_submatcher {
            if invert_next_matcher {
                top_level_matcher.push(Box::new(logical_matchers::NotMatcher::new(submatcher)));
                invert_next_matcher = false;
            } else {
                top_level_matcher.push(submatcher);
            }
        }
        i += 1;
    }
    if expecting_bracket {
        return Err(From::from("invalid expression; I was expecting to find a ')' somewhere but \
                               did not see one."));
    }
    Ok((i, Box::new(top_level_matcher)))
}

#[cfg(test)]
mod tests {
    use std::fs::DirEntry;
    use std::cell::RefCell;
    use std::vec::Vec;
    use std::io::Cursor;
    use std::rc::Rc;
    use std::io::Read;

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



    fn new_output() -> Rc<RefCell<Cursor<Vec<u8>>>> {
        Rc::new(RefCell::new(Cursor::new(Vec::<u8>::new())))
    }

    fn get_output_as_string(output: &RefCell<Cursor<Vec<u8>>>) -> String {
        let mut cursor = output.borrow_mut();
        cursor.set_position(0);
        let mut contents = String::new();
        cursor.read_to_string(&mut contents).unwrap();
        contents
    }

    #[test]
    fn build_top_level_matcher_name() {
        let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
        let abbbc_upper = get_dir_entry_for("./test_data/simple", "ABBBC");
        let output = new_output();

        let matcher = super::build_top_level_matcher(&["-name", "a*c"], output.clone()).unwrap();

        assert!(matcher.matches(&abbbc_lower));
        assert!(!matcher.matches(&abbbc_upper));
        assert_eq!(get_output_as_string(&output), "./test_data/simple/abbbc\n");
    }

    #[test]
    fn build_top_level_matcher_iname() {
        let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
        let abbbc_upper = get_dir_entry_for("./test_data/simple", "ABBBC");
        let output = new_output();

        let matcher = super::build_top_level_matcher(&["-iname", "a*c"], output.clone()).unwrap();

        assert!(matcher.matches(&abbbc_lower));
        assert!(matcher.matches(&abbbc_upper));
        assert_eq!(get_output_as_string(&output),
                   "./test_data/simple/abbbc\n./test_data/simple/ABBBC\n");
    }

    #[test]
    fn build_top_level_matcher_not() {
        for arg in &["-not", "!"] {
            let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
            let output = new_output();

            let matcher = super::build_top_level_matcher(&[arg, "-name", "doesntexist"],
                                                         output.clone())
                .unwrap();

            assert!(matcher.matches(&abbbc_lower));
            assert_eq!(get_output_as_string(&output), "./test_data/simple/abbbc\n");
        }
    }

    #[test]
    fn build_top_level_matcher_not_needs_expression() {
        for arg in &["-not", "!"] {
            let output = new_output();

            if let Err(e) = super::build_top_level_matcher(&[arg], output.clone()) {
                assert!(e.description().contains("expected an expression"));
            } else {
                panic!("parsing arugment lists that end in -not should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_missing_args() {
        for arg in &["-iname", "-name", "-type"] {
            let output = new_output();

            if let Err(e) = super::build_top_level_matcher(&[arg], output.clone()) {
                assert!(e.description().contains("missing argument to"));
                assert!(e.description().contains(arg));
            } else {
                panic!("parsing arugment lists that end in -not should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_or_without_expr1() {
        for arg in &["-or", "-o"] {
            let output = new_output();

            if let Err(e) = super::build_top_level_matcher(&[arg, "-true"], output.clone()) {
                assert!(e.description().contains("you have used a binary operator"));
            } else {
                panic!("parsing arugment list that begins with -or should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_or_without_expr2() {
        for arg in &["-or", "-o"] {
            let output = new_output();

            if let Err(e) = super::build_top_level_matcher(&["-true", arg], output.clone()) {
                assert!(e.description().contains("expected an expression"));
            } else {
                panic!("parsing arugment list that ends with -or should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_or_works() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        for args in &[["-true", "-o", "-false"],
                      ["-false", "-o", "-true"],
                      ["-true", "-o", "-true"]] {
            let output = new_output();

            let matcher = super::build_top_level_matcher(args, output.clone()).unwrap();

            assert!(matcher.matches(&abbbc));
            assert_eq!(get_output_as_string(&output), "./test_data/simple/abbbc\n");
        }

        let output = new_output();

        let matcher = super::build_top_level_matcher(&["-false", "-o", "-false"], output.clone())
            .unwrap();

        assert!(!matcher.matches(&abbbc));
        assert_eq!(get_output_as_string(&output), "");
    }

    #[test]
    fn build_top_level_matcher_and_works() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        for args in &[["-true", "-false"], ["-false", "-true"], ["-false", "-false"]] {
            let output = new_output();

            let matcher = super::build_top_level_matcher(args, output.clone()).unwrap();

            assert!(!matcher.matches(&abbbc));
            assert_eq!(get_output_as_string(&output), "");
        }

        let output = new_output();

        let matcher = super::build_top_level_matcher(&["-true", "-true"], output.clone()).unwrap();

        assert!(matcher.matches(&abbbc));
        assert_eq!(get_output_as_string(&output), "./test_data/simple/abbbc\n");
    }

    #[test]
    fn build_top_level_matcher_list_works() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        let args = ["-true", "-print", "-false", ",", "-print", "-false"];
        let output = new_output();

        let matcher = super::build_top_level_matcher(&args, output.clone()).unwrap();

        // final matcher returns false, so list matcher should too
        assert!(!matcher.matches(&abbbc));
        // two print matchers means doubled output
        assert_eq!(get_output_as_string(&output),
                   "./test_data/simple/abbbc\n./test_data/simple/abbbc\n");
    }

    #[test]
    fn build_top_level_matcher_list_without_expr1() {
        let output = new_output();

        if let Err(e) = super::build_top_level_matcher(&[",", "-true"], output.clone()) {
            assert!(e.description().contains("you have used a binary operator"));
        } else {
            panic!("parsing arugment list that begins with , should fail");
        }

        if let Err(e) = super::build_top_level_matcher(&["-true", "-o", ",", "-true"],
                                                       output.clone()) {
            assert!(e.description().contains("you have used a binary operator"));
        } else {
            panic!("parsing arugment list that contains '-o  ,' should fail");
        }

    }

    #[test]
    fn build_top_level_matcher_list_without_expr2() {
        let output = new_output();

        if let Err(e) = super::build_top_level_matcher(&["-true", ","], output.clone()) {
            assert!(e.description().contains("expected an expression"));
        } else {
            panic!("parsing arugment list that ends with , should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_not_enough_brackets() {
        let output = new_output();

        if let Err(e) = super::build_top_level_matcher(&["-true", "("], output.clone()) {
            assert!(e.description().contains("I was expecting to find a ')'"));
        } else {
            panic!("parsing arugment list with not enough closing brackets should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_too_many_brackets() {
        let output = new_output();

        if let Err(e) = super::build_top_level_matcher(&["-true", "(", ")", ")"], output.clone()) {
            assert!(e.description().contains("too many ')'"));
        } else {
            panic!("parsing arugment list with too many closing brackets should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_can_use_bracket_as_arg() {
        let output = new_output();
        // make sure that if we use a bracket as an argument (e.g. to -name)
        // then it isn't viewed as a bracket
        super::build_top_level_matcher(&["-name", "("], output.clone()).unwrap();
        super::build_top_level_matcher(&["-name", ")"], output.clone()).unwrap();
    }

    #[test]
    fn build_top_level_matcher_brackets_work() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        // same as true | ( false & false) = true
        let args_without = ["-true", "-o", "-false", "-false"];
        // same as (true | false) & false = false
        let args_with = ["(", "-true", "-o", "-false", ")", "-false"];
        let output = new_output();

        {
            let matcher = super::build_top_level_matcher(&args_without, output.clone()).unwrap();
            assert!(matcher.matches(&abbbc));
        }
        {
            let matcher = super::build_top_level_matcher(&args_with, output.clone()).unwrap();
            assert!(!matcher.matches(&abbbc));
        }
    }

    #[test]
    fn build_top_level_matcher_not_and_brackets_work() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        // same as (true & !(false)) | true = true
        let args_without = ["-true", "-not", "-false", "-o", "-true"];
        // same as true & !(false | true) = false
        let args_with = ["-true", "-not", "(", "-false", "-o", "-true", ")"];
        let output = new_output();

        {
            let matcher = super::build_top_level_matcher(&args_without, output.clone()).unwrap();
            assert!(matcher.matches(&abbbc));
        }
        {
            let matcher = super::build_top_level_matcher(&args_with, output.clone()).unwrap();
            assert!(!matcher.matches(&abbbc));
        }
    }

}
