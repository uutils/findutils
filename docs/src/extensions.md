# Extensions

The goal of this project is to be a drop-in replacement for GNU findutils, so by
default the tools behave exactly like the originals. A few opt-in extras go
beyond what GNU offers; they are all off unless you ask for them, and none of
them change the default output.

The sections below are the only ones. Nothing else here is an addition to GNU:
every other predicate, option, `-printf` directive and environment variable
either matches GNU or is still missing. In particular `LOCATE_PATH`,
`FINDOPTIONS`, `PRUNEPATHS`, `PRUNEFS`, `NETPATHS`, `LOCALUSER` and `NETUSER`
are standard findutils variables, not extensions. Where we differ from GNU it is
a gap rather than an extra — see [GNU test coverage](test_coverage.md).

## `-sorted`: deterministic traversal order

GNU `find` returns entries in whatever order the filesystem hands them back, so
two runs over the same tree can print the same lines in a different order. The
`-sorted` predicate makes `find` sort each directory's entries by name before
descending:

```console
$ find /srv/data -type f -printf '%f\n'
yankee
bravo
mike
alpha
zeta

$ find /srv/data -sorted -type f -printf '%f\n'
alpha
bravo
mike
yankee
zeta
```

It is a global flag rather than a test: it always matches, and it affects the
whole traversal no matter where it appears in the expression. Sorting requires
reading each directory in full before descending into it, so it costs memory and
latency on very large directories — that is why it is opt-in rather than the
default.

This is most useful when you want reproducible output: comparing two trees,
generating a manifest, or writing a test whose expected output is a fixed list
of lines.

## Rich expression diagnostics

`find` expressions get long, and a lone error line does not say *where* in the
expression the problem is. Down a pipe — a script, a test suite — that lone
line is all you get, because that is what such callers parse:

```console
$ find /srv/www -type f -a \( -name '*.php' -o -nmae '*.inc' \) -print 2>&1 | cat
find: unknown predicate `-nmae'
```

At a terminal, where the reader is a person, `find` underlines the argument at
fault in place, on the command line you actually typed:

```console
$ find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
   ╭─[ find:1:60 ]
   │
 1 │ find /srv/www -type f -a ( -name *.php -o -name *.phtml -o -nmae *.inc ) -print
   │                                                            ──┬──
   │                                                              ╰──── not a known predicate
   │
   │ Help: did you mean `-name'?
───╯
```

The first line is the message `find` has always printed, so anything that matched
it before still matches. Everything below it is added context.

### What gets a diagnostic

Errors in the *structure* of the expression, and in the predicates themselves.
The unknown-predicate case is shown above; the rest follow, all with
`UUTILS_DIAG=always` set in the environment.

A predicate left without its argument:

```console
$ find /home -xdev -type f -size +1G -a -mtime -7 -printf
find: missing argument to `-printf'
   ╭─[ find:1:49 ]
   │
 1 │ find /home -xdev -type f -size +1G -a -mtime -7 -printf
   │                                                 ───┬───
   │                                                    ╰───── this predicate needs an argument
───╯
```

An operator with nothing *after* it, typically a half-finished edit:

```console
$ find /var/spool -type f -a \( -user postfix -o -group mail \) -a -mtime +7 -o
find: expected an expression after '-o'
   ╭─[ find:1:74 ]
   │
 1 │ find /var/spool -type f -a ( -user postfix -o -group mail ) -a -mtime +7 -o
   │                                                                          ─┬
   │                                                                           ╰── nothing follows this operator
───╯
```

The same slip inside a group, where it is the `)` that cuts the operator off:

```console
$ find /var/log -type f \( -name '*.gz' -o -name '*.old' -o \) -print
find: expected an expression between '-o' and ')'
   ╭─[ find:1:53 ]
   │
 1 │ find /var/log -type f ( -name *.gz -o -name *.old -o ) -print
   │                                                     ─┬
   │                                                      ╰── nothing between this operator and the ')'
───╯
```

An operator with nothing *before* it. Note this underlines the leading token,
where the previous example underlined the trailing one — two mistakes that read
identically in the plain message now look different:

```console
$ find . -o -type f -name '*.tmp' -print
find: invalid expression; you have used a binary operator '-o' with nothing before it.
   ╭─[ find:1:8 ]
   │
 1 │ find . -o -type f -name *.tmp -print
   │        ─┬
   │         ╰── no expression before this operator
───╯
```

A `(` that is never closed. There are three `(` on this line; the diagnostic
picks the unbalanced one rather than pointing at the end of the command:

```console
$ find . \( -type d -a \( -name .git -o -name target \) -prune \) -o \( -type f -print
find: invalid expression; I was expecting to find a ')' somewhere but did not see one.
   ╭─[ find:1:64 ]
   │
 1 │ find . ( -type d -a ( -name .git -o -name target ) -prune ) -o ( -type f -print
   │                                                                ┬
   │                                                                ╰── this parenthesis is never closed
───╯
```

An empty group:

```console
$ find . -type f \( \) -o -name '*.bak' -print
find: invalid expression; empty parentheses are not allowed.
   ╭─[ find:1:16 ]
   │
 1 │ find . -type f ( ) -o -name *.bak -print
   │                ┬
   │                ╰── nothing between these parentheses
───╯
```

And a `)` with no opener:

```console
$ find /etc \( -name '*.conf' -a -newer /etc/fstab \) \) -o -name '*.cfg' -print
find: you have too many ')'
   ╭─[ find:1:49 ]
   │
 1 │ find /etc ( -name *.conf -a -newer /etc/fstab ) ) -o -name *.cfg -print
   │                                                 ┬
   │                                                 ╰── no matching '(' before this
───╯
```

Errors in the *value* of an argument — a bad `-size` suffix, `-perm` mode,
`-type` list, `-printf` format or `-newerXY` date — keep the plain single-line
message for now.

### Comparison with GNU

GNU `find` reports the same errors, but only ever as a single line. On a typo
buried in a group, that is all you get:

```console
$ find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
```

```console
$ UUTILS_DIAG=always find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
   ╭─[ find:1:60 ]
   │
 1 │ find /srv/www -type f -a ( -name *.php -o -name *.phtml -o -nmae *.inc ) -print
   │                                                            ──┬──
   │                                                              ╰──── not a known predicate
   │
   │ Help: did you mean `-name'?
───╯
```

The difference is starkest where the message names no argument at all. GNU tells
you a `)` is missing, but not which `(` is unbalanced — on a line with three of
them, that is the whole question:

```console
$ find . \( -type d -a \( -name .git -o -name target \) -prune \) -o \( -type f -print
find: invalid expression; I was expecting to find a ')' somewhere but did not see one.
```

```console
$ UUTILS_DIAG=always find . \( -type d -a \( -name .git -o -name target \) -prune \) -o \( -type f -print
find: invalid expression; I was expecting to find a ')' somewhere but did not see one.
   ╭─[ find:1:64 ]
   │
 1 │ find . ( -type d -a ( -name .git -o -name target ) -prune ) -o ( -type f -print
   │                                                                ┬
   │                                                                ╰── this parenthesis is never closed
───╯
```

In both cases the first line is identical to what GNU 4.10.0 prints, and the exit
code is `1` either way. The report only ever *adds* the block below.

`UUTILS_DIAG` overrides the terminal check in either direction: set it to
`never` to always get the single line, or to `always` to get the report even
when standard error is redirected. Values are matched case-insensitively;
anything else — including an empty value — counts as unset and leaves the
terminal check in charge. It is the same variable, with the same values, that
the uutils coreutils read.

Every message is byte-for-byte identical to GNU's under `LC_ALL=C`:

```text
unknown predicate `-nmae'
missing argument to `-printf'
expected an expression after '-o'
expected an expression between '-o' and ')'
invalid expression; you have used a binary operator '-o' with nothing before it.
invalid expression; empty parentheses are not allowed.
invalid expression; I was expecting to find a ')' somewhere but did not see one.
you have too many ')'
```

### Notes

- **The rendered line is a reconstruction, not an echo.** It shows the arguments
  `find` was handed, after the shell had its say: `\(` and `'*.php'` arrive as
  `(` and `*.php` and are shown that way. Only an argument holding whitespace, or
  an empty one, is quoted, so that the underline still lines up with a word. That
  is what makes the underline trustworthy even when the shell rewrote what you
  typed.
- **Suggestions are edit-distance based**, with a threshold that scales with the
  length of the name (one edit up to 3 characters, two up to 7, three beyond),
  and a second rule that an edit may never rewrite half of what was typed.
  Transpositions and dropped letters are caught — `-nam`, `-pritn`, `-exce`,
  `-mindpeth` — while genuinely unrelated input gets no suggestion rather than a
  misleading one. The half rule is what keeps a misplaced `-H` or `-P` from
  being "corrected" to an unrelated two-character predicate it happens to sit
  one edit away from.
- **Colour follows [`NO_COLOR`](https://no-color.org)** and is used only when
  standard error is a terminal. A report forced on with `UUTILS_DIAG=always`
  still goes wherever standard error points, so it stays plain when that is a
  file.
- **On Windows the report heading reads `find.exe:`**, where the plain line
  reads `find:`: the heading comes from uucore, which keeps the `.exe`.
- **Nothing changes for a non-terminal standard error**, which is how the GNU
  and bfs compatibility testsuites — and every script — run `find`. Standard
  error stays byte-for-byte what it was.
