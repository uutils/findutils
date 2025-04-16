// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let strs: Vec<&str> = args.iter().map(std::convert::AsRef::as_ref).collect();
    std::process::exit(findutils::updatedb::updatedb_main(strs.as_slice()));
}
