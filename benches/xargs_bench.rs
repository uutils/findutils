// Copyright 2024 the uutils developers
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! End-to-end benchmarks for `xargs`, run through the real `xargs_main` entry
//! point. Input is supplied via `-a <file>` (rather than stdin) so the harness
//! can feed a fixed corpus without touching the process's stdin, and the
//! command is `true` so the measurement is dominated by `xargs`'s own work
//! (reading, splitting and batching arguments) rather than the child.

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use findutils::xargs::xargs_main;

/// Run `xargs` end-to-end. `args` are the arguments after the program name.
/// The exit status is ignored — we only care about the work performed.
fn run(args: &[&str]) {
    let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 1);
    argv.push("xargs");
    argv.extend_from_slice(args);
    let _ = xargs_main(&argv);
}

/// Write `count` short tokens to a temp file, joined by `sep`. Returns the path.
fn build_input(count: u32, sep: &str, tag: &str) -> PathBuf {
    let mut content = String::new();
    for i in 0..count {
        if i > 0 {
            content.push_str(sep);
        }
        content.push_str(&format!("item_{i}"));
    }
    let path = std::env::temp_dir().join(format!("uu_xargs_bench_{}_{tag}", std::process::id()));
    std::fs::write(&path, content).unwrap();
    path
}

fn bench_e2e(c: &mut Criterion) {
    // Whitespace- and newline-separated corpora, plus a NUL-separated one for
    // the `-0` path. A few thousand short tokens keeps parsing dominant while
    // `true` batches stay to a handful of (untraced) spawns.
    let ws = build_input(4000, " ", "ws");
    let nul = build_input(4000, "\0", "nul");
    let ws_path = ws.to_str().unwrap();
    let nul_path = nul.to_str().unwrap();

    let mut group = c.benchmark_group("xargs");

    // Default whitespace splitting, single batch (fits one command line).
    group.bench_function("split_whitespace", |b| {
        b.iter(|| run(black_box(&["-a", ws_path, "true"])));
    });
    // NUL-delimited input (`find -print0 | xargs -0` shape).
    group.bench_function("split_null", |b| {
        b.iter(|| run(black_box(&["-0", "-a", nul_path, "true"])));
    });
    // Cap arguments per command with -n, exercising the batching path across
    // several (untraced) invocations.
    group.bench_function("batched_n", |b| {
        b.iter(|| run(black_box(&["-a", ws_path, "-n", "1000", "true"])));
    });
    // Bound each command line by size with -s, another batching trigger.
    group.bench_function("batched_size", |b| {
        b.iter(|| run(black_box(&["-a", ws_path, "-s", "4096", "true"])));
    });

    group.finish();

    let _ = std::fs::remove_file(&ws);
    let _ = std::fs::remove_file(&nul);
}

criterion_group!(benches, bench_e2e);
criterion_main!(benches);
