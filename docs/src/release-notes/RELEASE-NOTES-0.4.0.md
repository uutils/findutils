# 📦 Findutils 0.4.0 Release

## What's Changed
* Make multi-exec only match {} + with nothing in between by @tavianator in https://github.com/uutils/findutils/pull/91
* Upgrade to GitHub-native Dependabot by @dependabot-preview in https://github.com/uutils/findutils/pull/94
* exec: Handle parent directories more carefully by @tavianator in https://github.com/uutils/findutils/pull/92
* Add support for GNU-compatible printf by @refi64 in https://github.com/uutils/findutils/pull/120
* Add initial parts of automated compatibility tests by @refi64 in https://github.com/uutils/findutils/pull/129
* Add remaining portions of automated compatibility tests by @refi64 in https://github.com/uutils/findutils/pull/130
* Add support for -print0 by @refi64 in https://github.com/uutils/findutils/pull/123
* Add support for regex matching by @refi64 in https://github.com/uutils/findutils/pull/126
* Add an initial implementation of xargs by @refi64 in https://github.com/uutils/findutils/pull/121
* Fix compat tests not using the latest workflow by @refi64 in https://github.com/uutils/findutils/pull/133
* Add support for -lname / -ilname by @refi64 in https://github.com/uutils/findutils/pull/138
* Add support for -empty by @refi64 in https://github.com/uutils/findutils/pull/137
* Add support for -xdev by @refi64 in https://github.com/uutils/findutils/pull/136
* Avoid skipping the entire directory if a file hits -prune by @refi64 in https://github.com/uutils/findutils/pull/139
* matchers: Replace new_box() with an into_box() trait method by @tavianator in https://github.com/uutils/findutils/pull/141
* printf: Fix some time formatting to match GNU find by @tavianator in https://github.com/uutils/findutils/pull/146
* Support -and as a synonym for -a by @tavianator in https://github.com/uutils/findutils/pull/145
* Implement -P and -- by @tavianator in https://github.com/uutils/findutils/pull/148
* Implement -quit by @tavianator in https://github.com/uutils/findutils/pull/147
* POSIX compliant globs by @tavianator in https://github.com/uutils/findutils/pull/151
* find: Use the uucore mode parsing implementation for -perm by @tavianator in https://github.com/uutils/findutils/pull/154
* find/matchers: Implement -mount as an alias for -xdev by @tavianator in https://github.com/uutils/findutils/pull/165
* find/matchers: Add the `ed` and `sed` regex types by @tavianator in https://github.com/uutils/findutils/pull/166
* find/matchers: Implement the -{read,writ,execut}able access checks by @tavianator in https://github.com/uutils/findutils/pull/168
* find/matchers: Implement -inum and -links by @tavianator in https://github.com/uutils/findutils/pull/167
* find: Don't swallow mkfifo errors in the tests by @refi64 in https://github.com/uutils/findutils/pull/179
* Add support for embedded "{}" by @int3 in https://github.com/uutils/findutils/pull/213

## Code quality
* Various clippy fixes + precommit by @sylvestre in https://github.com/uutils/findutils/pull/222
* Fix a clippy warning by @sylvestre in https://github.com/uutils/findutils/pull/188
* Fix some clippy warnings by @sylvestre in https://github.com/uutils/findutils/pull/103
* Add back coverage with codecov  by @sylvestre in https://github.com/uutils/findutils/pull/111
* Various Clippy fixes by @sylvestre in https://github.com/uutils/findutils/pull/143

## CI
* ci: Update BFS testsuite to version 2.4 by @tavianator in https://github.com/uutils/findutils/pull/150
* ci: Update bfs testsuite to version 2.6 by @tavianator in https://github.com/uutils/findutils/pull/163
* ci: Also run the dejagnu tests from GNU findutils by @tavianator in https://github.com/uutils/findutils/pull/144
* run the GNU testsuite in the CI by @sylvestre in https://github.com/uutils/findutils/pull/115
* ci: Run the bfs testsuite by @tavianator in https://github.com/uutils/findutils/pull/116
* fix the ci warnings by @sylvestre in https://github.com/uutils/findutils/pull/211

## Dependencies
* update predicates by @sylvestre in https://github.com/uutils/findutils/pull/104
* Move tempfile dep to dev-dep by @sylvestre in https://github.com/uutils/findutils/pull/135
* replace tempdir by tempfile by @sylvestre in https://github.com/uutils/findutils/pull/110
* Bump once_cell from 1.9.0 to 1.10.0 by @dependabot in https://github.com/uutils/findutils/pull/152
* build(deps): bump serial_test from 0.6.0 to 0.7.0 by @dependabot in https://github.com/uutils/findutils/pull/169
* build(deps): bump serial_test from 0.7.0 to 0.8.0 by @dependabot in https://github.com/uutils/findutils/pull/170
* build(deps): bump filetime from 0.2.16 to 0.2.17 by @dependabot in https://github.com/uutils/findutils/pull/172
* build(deps): bump onig from 6.3.1 to 6.3.2 by @dependabot in https://github.com/uutils/findutils/pull/171
* build(deps): bump once_cell from 1.12.0 to 1.13.0 by @dependabot in https://github.com/uutils/findutils/pull/173
* build(deps): bump regex from 1.5.6 to 1.6.0 by @dependabot in https://github.com/uutils/findutils/pull/174
* build(deps): bump onig from 6.3.2 to 6.4.0 by @dependabot in https://github.com/uutils/findutils/pull/176
* build(deps): bump serial_test from 0.8.0 to 0.9.0 by @dependabot in https://github.com/uutils/findutils/pull/178
* build(deps): bump nix from 0.24.2 to 0.25.0 by @dependabot in https://github.com/uutils/findutils/pull/180
* build(deps): bump chrono from 0.4.19 to 0.4.20 by @dependabot in https://github.com/uutils/findutils/pull/175
* build(deps): bump once_cell from 1.13.0 to 1.13.1 by @dependabot in https://github.com/uutils/findutils/pull/182
* build(deps): bump chrono from 0.4.20 to 0.4.22 by @dependabot in https://github.com/uutils/findutils/pull/181
* build(deps): bump iana-time-zone from 0.1.44 to 0.1.47 by @dependabot in https://github.com/uutils/findutils/pull/183
* build(deps): bump once_cell from 1.13.1 to 1.15.0 by @dependabot in https://github.com/uutils/findutils/pull/185
* build(deps): bump assert_cmd from 2.0.4 to 2.0.5 by @dependabot in https://github.com/uutils/findutils/pull/187
* build(deps): bump predicates from 2.1.5 to 3.0.2 by @dependabot in https://github.com/uutils/findutils/pull/220
* build(deps): bump filetime from 0.2.17 to 0.2.18 by @dependabot in https://github.com/uutils/findutils/pull/186
* ci: Update bfs to 2.6.2 by @tavianator in https://github.com/uutils/findutils/pull/189
* build(deps): bump once_cell from 1.15.0 to 1.16.0 by @dependabot in https://github.com/uutils/findutils/pull/190
* build(deps): bump predicates from 2.1.1 to 2.1.2 by @dependabot in https://github.com/uutils/findutils/pull/191
* build(deps): bump assert_cmd from 2.0.5 to 2.0.6 by @dependabot in https://github.com/uutils/findutils/pull/193
* build(deps): bump regex from 1.6.0 to 1.7.0 by @dependabot in https://github.com/uutils/findutils/pull/192
* build(deps): bump predicates from 2.1.2 to 2.1.3 by @dependabot in https://github.com/uutils/findutils/pull/194
* build(deps): bump chrono from 0.4.22 to 0.4.23 by @dependabot in https://github.com/uutils/findutils/pull/195
* build(deps): bump nix from 0.25.0 to 0.26.1 by @dependabot in https://github.com/uutils/findutils/pull/196
* build(deps): bump filetime from 0.2.18 to 0.2.19 by @dependabot in https://github.com/uutils/findutils/pull/199
* build(deps): bump assert_cmd from 2.0.6 to 2.0.7 by @dependabot in https://github.com/uutils/findutils/pull/198
* build(deps): bump predicates from 2.1.3 to 2.1.4 by @dependabot in https://github.com/uutils/findutils/pull/197
* build(deps): bump serial_test from 0.9.0 to 0.10.0 by @dependabot in https://github.com/uutils/findutils/pull/200
* build(deps): bump predicates from 2.1.4 to 2.1.5 by @dependabot in https://github.com/uutils/findutils/pull/202
* build(deps): bump once_cell from 1.16.0 to 1.17.0 by @dependabot in https://github.com/uutils/findutils/pull/201
* build(deps): bump assert_cmd from 2.0.7 to 2.0.8 by @dependabot in https://github.com/uutils/findutils/pull/204
* build(deps): bump regex from 1.7.0 to 1.7.1 by @dependabot in https://github.com/uutils/findutils/pull/203
* build(deps): bump nix from 0.26.1 to 0.26.2 by @dependabot in https://github.com/uutils/findutils/pull/205
* build(deps): bump serial_test from 0.10.0 to 1.0.0 by @dependabot in https://github.com/uutils/findutils/pull/206
* build(deps): bump bumpalo from 3.10.0 to 3.12.0 by @dependabot in https://github.com/uutils/findutils/pull/207
* build(deps): bump once_cell from 1.17.0 to 1.17.1 by @dependabot in https://github.com/uutils/findutils/pull/209
* Bump walkdir from 2.3.1 to 2.3.2 by @dependabot-preview in https://github.com/uutils/findutils/pull/90
* Bump regex from 1.4.5 to 1.5.3 by @dependabot-preview in https://github.com/uutils/findutils/pull/96
* Bump assert_cmd from 1.0.3 to 1.0.4 by @dependabot in https://github.com/uutils/findutils/pull/98
* Bump regex from 1.5.3 to 1.5.4 by @dependabot in https://github.com/uutils/findutils/pull/97
* Bump predicates from 1.0.7 to 1.0.8 by @dependabot-preview in https://github.com/uutils/findutils/pull/95
* Bump assert_cmd from 1.0.4 to 1.0.5 by @dependabot in https://github.com/uutils/findutils/pull/99
* Bump assert_cmd from 1.0.5 to 1.0.8 by @dependabot in https://github.com/uutils/findutils/pull/106
* Bump assert_cmd from 1.0.8 to 2.0.0 by @dependabot in https://github.com/uutils/findutils/pull/107
* Bump predicates from 2.0.0 to 2.0.1 by @dependabot in https://github.com/uutils/findutils/pull/105
* Bump predicates from 2.0.1 to 2.0.2 by @dependabot in https://github.com/uutils/findutils/pull/108
* Bump assert_cmd from 2.0.0 to 2.0.1 by @dependabot in https://github.com/uutils/findutils/pull/109
* Update uucore to 0.0.12 by @refi64 in https://github.com/uutils/findutils/pull/132
* Bump serial_test from 0.5.1 to 0.6.0 by @dependabot in https://github.com/uutils/findutils/pull/149
* Bump assert_cmd from 2.0.1 to 2.0.2 by @dependabot in https://github.com/uutils/findutils/pull/117
* Bump predicates from 2.0.2 to 2.1.0 by @dependabot in https://github.com/uutils/findutils/pull/119
* Bump tempfile from 3.2.0 to 3.3.0 by @dependabot in https://github.com/uutils/findutils/pull/122
* Bump predicates from 2.1.0 to 2.1.1 by @dependabot in https://github.com/uutils/findutils/pull/124
* Bump assert_cmd from 2.0.2 to 2.0.4 by @dependabot in https://github.com/uutils/findutils/pull/127
* Bump regex from 1.5.4 to 1.5.5 by @dependabot in https://github.com/uutils/findutils/pull/153
* Bump filetime from 0.2.15 to 0.2.16 by @dependabot in https://github.com/uutils/findutils/pull/156
* Bump once_cell from 1.10.0 to 1.11.0 by @dependabot in https://github.com/uutils/findutils/pull/157
* Bump actions/upload-artifact from 2 to 3 by @dependabot in https://github.com/uutils/findutils/pull/158
* Bump actions/checkout from 2 to 3 by @dependabot in https://github.com/uutils/findutils/pull/159
* Bump codecov/codecov-action from 1 to 3 by @dependabot in https://github.com/uutils/findutils/pull/160
* Bump regex from 1.5.5 to 1.5.6 by @dependabot in https://github.com/uutils/findutils/pull/161
* build(deps): bump once_cell from 1.11.0 to 1.12.0 by @dependabot in https://github.com/uutils/findutils/pull/162

## New Contributors
* @tavianator made their first contribution in https://github.com/uutils/findutils/pull/91
* @dependabot made their first contribution in https://github.com/uutils/findutils/pull/98
* @sylvestre made their first contribution in https://github.com/uutils/findutils/pull/103
* @refi64 made their first contribution in https://github.com/uutils/findutils/pull/120
* @int3 made their first contribution in https://github.com/uutils/findutils/pull/213

**Full Changelog**: https://github.com/uutils/findutils/compare/0.1.0...0.4.0
