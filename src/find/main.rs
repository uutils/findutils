extern crate glob;
extern crate findutils;

fn main() {
    let ref args: &Vec<String> = &std::env::args().collect();
    std::process::exit(findutils::find::find_main(args));
}
