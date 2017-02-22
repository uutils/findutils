extern crate glob;
extern crate findutils;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let deps = findutils::find::StandardDependencies::new();
    std::process::exit(findutils::find::find_main(&strs, &deps));
}
