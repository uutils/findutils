// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

extern crate glob;
extern crate findutils;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let deps = findutils::find::StandardDependencies::new();
    std::process::exit(findutils::find::find_main(&strs, &deps));
}
