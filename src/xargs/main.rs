// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

fn main() {
    // `std::env::args` panics on a non-UTF-8 argument, so read the arguments
    // with `args_os` and report the first invalid one instead of aborting.
    let args = match std::env::args_os()
        .map(std::ffi::OsString::into_string)
        .collect::<Result<Vec<String>, _>>()
    {
        Ok(args) => args,
        Err(invalid) => {
            eprintln!(
                "xargs: invalid (non-UTF-8) argument: {}",
                invalid.to_string_lossy()
            );
            std::process::exit(1);
        }
    };
    std::process::exit(findutils::xargs::xargs_main(
        &args
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<&str>>(),
    ))
}
