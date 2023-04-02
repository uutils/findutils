// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    std::process::exit(findutils::xargs::xargs_main(
        &args
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<&str>>(),
    ))
}
