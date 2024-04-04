// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

fn main() {
    // Ignores the SIGPIPE signal.
    // This is to solve the problem that when find is used with a pipe character,
    // the downstream software of the standard output stream closes the pipe and triggers a panic.
    uucore::panic::mute_sigpipe_panic();

    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(std::convert::AsRef::as_ref).collect();
    let deps = findutils::find::StandardDependencies::new();
    std::process::exit(findutils::find::find_main(&strs, &deps));
}
