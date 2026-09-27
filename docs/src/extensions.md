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
