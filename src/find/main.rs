extern crate glob;
extern crate findutils;

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let out: Rc<RefCell<Write>> = Rc::new(RefCell::new(io::stdout()));
    std::process::exit(findutils::find::find_main(&strs, out));
}
