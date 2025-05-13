// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[cfg(not(windows))]
fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(std::convert::AsRef::as_ref).collect();
    std::process::exit(findutils::locate::locate_main(strs.as_slice()));
}

#[cfg(windows)]
fn main() {
    // TODO: locate currently uses UNIX-specific OsString APIs. If those can be worked around, locate
    // should function normally on Windows. If and when that happens, make sure to make a separate
    // windows test database with \ instead of /.
    println!("locate is unsupported on Windows");
}
