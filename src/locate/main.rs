fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(std::convert::AsRef::as_ref).collect();
    std::process::exit(findutils::locate::locate_main(strs.as_slice()));
}
