# AGENTS.md

Writing this file is not a statement for or against using AI coding agents.
People are using them in this repository either way, so this file exists to make
sure that when they do, the project's rules are actually followed.

If you are driving an agent here, you are responsible for its output. Read the
diff before you send it.

Responses to reviewers should be written by a human, not an agent.

## Start with an issue

Before opening a PR, search for an existing issue describing the problem being
addressed. Search both open and closed issues where relevant.

When an existing issue covers the work, reference it in the PR.

If you have already opened a PR for the work, continue working on that PR
rather than opening a new one to replace it. Rebase or otherwise update the
existing PR's branch as needed.

If the PR fully resolves an issue, include `Closes #XXX` in the PR
description so that the issue is automatically closed when the PR is merged.

## Never read or copy GNU code

GNU findutils is GPLv3; this project is MIT. Any code derived from it - even a
few lines, a helper structure, a test fixture or a comment - cannot be accepted.
Do not link to the GNU source either. Reading the GNU manuals and comparing
against the behavior of the GNU binaries is fine, and so is running the GNU
test suite to check how a utility should behave.

## Don't duplicate code

Agents are very good at duplicating code: copying a helper, re-implementing a
parser that already exists, or pasting the same logic in a second place with a
small tweak. Reviewers will ask you to remove it.

- Before writing a helper, search this repository for an existing one and reuse
  or extend it.
- If two places need the same logic, factor it out instead of copying it.
- Tests too: share fixtures and helpers rather than repeating the same setup.

Code that can be useful to other uutils projects (option parsing, error
reporting, file system helpers, locale handling, ...) belongs in
[`uucore`](https://github.com/uutils/coreutils/tree/main/src/uucore), the
shared crate maintained in uutils/coreutils. Check whether `uucore` already
provides what you need before adding it here, and if it doesn't, consider
sending the change to `uucore` so every uutils project can use it.

## A PR needs tests

New behavior or a bug fix comes with a test, in `tests/test_find.rs`,
`tests/test_xargs.rs`, `tests/find_exec_tests.rs` or `tests/db_tests.rs`
(locate/updatedb), or as a unit test next to the code. If a GNU test used to
fail and now passes (`util/build-gnu.sh`), add a Rust test so it cannot
silently regress.

No tests, no merge.

Code coverage and performance must not regress. CI reports both: check the
coverage report and the CodSpeed results on your PR. When touching a hot path,
run or extend the benchmarks in `benches/` (`cargo bench`).

## Keep the PR description short

Describe the problem being solved and what changed. That is all.

- No generated walls of text, no bullet-point summaries of every hunk, no
  emoji-headed sections.
- Title: `<util>: <what changed>`, e.g. `xargs: with -x, stop before running the pending command`.
- Write issue reports, PR descriptions and replies to reviewers in your own
  words. The point of review is to check that a human understands the change.

## Read the docs already in this repo

Before changing anything, look at the Markdown files here - they are the actual
rules, this file is only a pointer:

- `CONTRIBUTING.md` - licensing, Rust style, commit hygiene, what a PR must contain
- `DEVELOPMENT.md` - build, test and environment setup
- `README.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`
- `docs/src/*.md` - extensions, platforms, test coverage
