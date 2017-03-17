// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.


// As this module is included by all the integration tests, any function used
// in one test but not another can cause a dead code warning.
#[allow(dead_code)]
pub mod test_helpers;
