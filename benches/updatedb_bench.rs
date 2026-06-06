// Copyright 2024 the uutils developers
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! End-to-end benchmarks for `updatedb`, run through the real `updatedb_main`
//! entry point. Each case walks a generated directory tree (via the internal
//! `find`), encodes it into the LOCATE02 database format and writes the result
//! to a temp file, so the walk + front-coding + write dominate the measurement.

use std::path::{Path, PathBuf};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use findutils::updatedb::updatedb_main;

/// Run `updatedb` end-to-end. `args` are the arguments after the program name.
fn run(args: &[&str]) {
    let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 1);
    argv.push("updatedb");
    argv.extend_from_slice(args);
    let _ = updatedb_main(&argv);
}

/// Build a directory tree to index. `depth` levels, each holding `dirs`
/// sub-directories and `files` files with varied names. Returns the root.
fn build_tree(depth: u32, dirs: u32, files: u32) -> PathBuf {
    let root = std::env::temp_dir().join(format!("uu_updatedb_bench_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    populate(&root, depth, dirs, files, 0);
    root
}

fn populate(dir: &Path, depth: u32, dirs: u32, files: u32, seed: u32) {
    for f in 0..files {
        let n = seed.wrapping_add(f);
        let name = match n % 4 {
            0 => format!("module_{n}.rs"),
            1 => format!("data_{n}.txt"),
            2 => format!("image_{n}.png"),
            _ => format!("file_{n}.log"),
        };
        // Tiny files: updatedb only records names, so content size is irrelevant.
        std::fs::write(dir.join(name), b"x").unwrap();
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
    // depth 4, 4 dirs/level, 25 files/dir → ~8.5k indexed paths.
    let root = build_tree(4, 4, 25);
    let dir = root.to_str().unwrap();
    let db = std::env::temp_dir().join(format!("uu_updatedb_bench_{}.db", std::process::id()));
    let db_path = db.to_str().unwrap();

    let localpaths = format!("--localpaths={dir}");
    let output = format!("--output={db_path}");

    let mut group = c.benchmark_group("updatedb");

    // Full build: walk, front-code and write the whole tree. Empty prune
    // options keep the run deterministic regardless of where temp_dir lives.
    group.bench_function("build_full", |b| {
        b.iter(|| {
            run(black_box(&[
                &localpaths,
                &output,
                "--prunepaths=",
                "--prunefs=",
            ]));
        });
    });

    // Build with a prune clause that drops every `dir_0` subtree, exercising the
    // -regex/-prune path updatedb hands to find.
    group.bench_function("build_pruned", |b| {
        b.iter(|| {
            run(black_box(&[
                &localpaths,
                &output,
                "--prunepaths=.*/dir_0",
                "--prunefs=",
            ]));
        });
    });

    group.finish();

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&db);
}

criterion_group!(benches, bench_e2e);
criterion_main!(benches);
