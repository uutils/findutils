// Copyright 2024 the uutils developers
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! End-to-end benchmarks for `find`, run through the real `find_main` entry
//! point so they exercise argument parsing, the matcher tree and the directory
//! walk together. Output is sent to a sink so the benchmarks measure the work
//! `find` does rather than terminal I/O.

use std::cell::RefCell;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use findutils::find::{find_main, Dependencies};

/// `Dependencies` implementation that throws output away. A fixed `now` keeps
/// time-based matchers (`-newer`, `-mtime`, …) deterministic across runs.
struct SinkDependencies {
    output: RefCell<io::Sink>,
    now: SystemTime,
}

impl SinkDependencies {
    fn new() -> Self {
        Self {
            output: RefCell::new(io::sink()),
            now: SystemTime::now(),
        }
    }
}

impl Dependencies for SinkDependencies {
    fn get_output(&self) -> &RefCell<dyn Write> {
        &self.output
    }

    fn now(&self) -> SystemTime {
        self.now
    }

    fn confirm(&self, _prompt: &str) -> bool {
        false
    }
}

/// Run `find` end-to-end. `args` are the arguments after the program name
/// (flags, paths, expression). The exit status is ignored — we only care about
/// the work performed.
fn run(args: &[&str]) {
    let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 1);
    argv.push("find");
    argv.extend_from_slice(args);
    let deps = SinkDependencies::new();
    let _ = find_main(&argv, &deps);
}

/// Build a moderately deep directory tree to walk. `depth` levels, each holding
/// `dirs` sub-directories and `files` files. File names vary so name/regex
/// matchers have something to discriminate on, and sizes vary so `-size` does
/// real work. Returns the root directory.
fn build_tree(depth: u32, dirs: u32, files: u32) -> PathBuf {
    let root = std::env::temp_dir().join(format!("uu_find_bench_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    populate(&root, depth, dirs, files, 0);
    root
}

fn populate(dir: &Path, depth: u32, dirs: u32, files: u32, seed: u32) {
    for f in 0..files {
        let n = seed.wrapping_add(f);
        // A mix of extensions and a sprinkling of a rare marker name.
        let name = match n % 5 {
            0 => format!("module_{n}.rs"),
            1 => format!("data_{n}.txt"),
            2 => format!("image_{n}.png"),
            3 => format!("README_{n}.md"),
            _ if n % 500 == 0 => format!("RAREHIT_{n}.log"),
            _ => format!("file_{n}.log"),
        };
        // Sizes from a few bytes up to ~8 KiB so -size buckets differ.
        let size = (n as usize % 8192) + 1;
        let path = dir.join(name);
        std::fs::write(&path, vec![b'x'; size]).unwrap();
    }

    if depth == 0 {
        return;
    }

    for d in 0..dirs {
        let sub = dir.join(format!("dir_{d}"));
        std::fs::create_dir_all(&sub).unwrap();
        populate(
            &sub,
            depth - 1,
            dirs,
            files,
            seed.wrapping_add((d + 1) * 31),
        );
    }
}

fn bench_e2e(c: &mut Criterion) {
    // depth 4, 4 dirs/level, 25 files/dir → a few thousand entries.
    let root = build_tree(4, 4, 25);
    let dir = root.to_str().unwrap();

    let mut group = c.benchmark_group("find");

    // Plain full walk with the implicit -print.
    group.bench_function("walk_all", |b| {
        b.iter(|| run(black_box(&[dir])));
    });
    // Filter by file type only.
    group.bench_function("type_f", |b| {
        b.iter(|| run(black_box(&[dir, "-type", "f"])));
    });
    // Glob name match — a common invocation.
    group.bench_function("name_glob", |b| {
        b.iter(|| run(black_box(&[dir, "-name", "*.rs"])));
    });
    // Case-insensitive name match.
    group.bench_function("iname_glob", |b| {
        b.iter(|| run(black_box(&[dir, "-iname", "*.RS"])));
    });
    // Regex over the whole path.
    group.bench_function("regex_path", |b| {
        b.iter(|| run(black_box(&[dir, "-regex", r".*/module_[0-9]+\.rs"])));
    });
    // Size predicate forces a stat per entry.
    group.bench_function("size_gt", |b| {
        b.iter(|| run(black_box(&[dir, "-type", "f", "-size", "+4k"])));
    });
    // Combined predicate with AND/OR and grouping.
    group.bench_function("combined_expr", |b| {
        b.iter(|| {
            run(black_box(&[
                dir, "-type", "f", "(", "-name", "*.rs", "-o", "-name", "*.md", ")",
            ]));
        });
    });
    // Prune whole subtrees, then print the rest.
    group.bench_function("prune", |b| {
        b.iter(|| {
            run(black_box(&[
                dir, "-name", "dir_0", "-prune", "-o", "-type", "f", "-print",
            ]));
        });
    });
    // -printf with several directives exercises the formatter.
    group.bench_function("printf", |b| {
        b.iter(|| run(black_box(&[dir, "-type", "f", "-printf", "%p %s %y\\n"])));
    });

    group.finish();

    let _ = std::fs::remove_dir_all(&root);
}

criterion_group!(benches, bench_e2e);
criterion_main!(benches);
