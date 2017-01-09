extern crate glob;
extern crate findutils;

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

fn main() {
    let ref args: &Vec<String> = &std::env::args().collect();
    let out: Rc<RefCell<Write>> = Rc::new(RefCell::new(io::stdout()));
    std::process::exit(findutils::find::find_main(args, out));
}
