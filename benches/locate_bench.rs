// Copyright 2024 the uutils developers
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! End-to-end benchmarks for `locate`, run through the real `locate_main` entry
//! point. A LOCATE02 database is built once (via `updatedb`) from a generated
//! tree, then each case scans it in `-c`/count mode so the database decoding
//! and pattern matching dominate rather than terminal output.
//!
//! `locate` is Unix-only (it relies on `OsStr`/`CStr` byte APIs), so the
//! benchmark body is compiled only there.

#[cfg(unix)]
mod bench {
    use std::path::{Path, PathBuf};

    use criterion::{black_box, criterion_group, Criterion};
    use findutils::locate::locate_main;
    use findutils::updatedb::updatedb_main;

    /// Run `locate` end-to-end. `args` are the arguments after the program name.
    fn run(args: &[&str]) {
        let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 1);
        argv.push("locate");
        argv.extend_from_slice(args);
        let _ = locate_main(&argv);
    }

    fn build_tree(depth: u32, dirs: u32, files: u32) -> PathBuf {
        let root = std::env::temp_dir().join(format!("uu_locate_bench_{}", std::process::id()));
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

    pub fn bench_e2e(c: &mut Criterion) {
        // ~8.5k paths, then front-coded into a LOCATE02 database once.
        let root = build_tree(4, 4, 25);
        let dir = root.to_str().unwrap();
        let db = std::env::temp_dir().join(format!("uu_locate_bench_{}.db", std::process::id()));
        let db_path = db.to_str().unwrap();

        let _ = updatedb_main(&[
            "updatedb",
            &format!("--localpaths={dir}"),
            &format!("--output={db_path}"),
            "--prunepaths=",
            "--prunefs=",
        ]);

        let mut group = c.benchmark_group("locate");

        // Pattern matching nothing: forces a full decode + match of every entry.
        group.bench_function("count_no_match", |b| {
            b.iter(|| run(black_box(&["-d", db_path, "-c", "NONEXISTENT_TOKEN_XYZ"])));
        });
        // Substring matching many entries (count mode → no per-match output).
        group.bench_function("count_many", |b| {
            b.iter(|| run(black_box(&["-d", db_path, "-c", ".log"])));
        });
        // Basename matching (-b) restricts the comparison to the file name.
        group.bench_function("basename", |b| {
            b.iter(|| run(black_box(&["-d", db_path, "-c", "-b", "module"])));
        });
        // Case-insensitive matching (-i).
        group.bench_function("ignore_case", |b| {
            b.iter(|| run(black_box(&["-d", db_path, "-c", "-i", "MODULE"])));
        });
        // Regex matching (-r) over the whole path.
        group.bench_function("regex", |b| {
            b.iter(|| {
                run(black_box(&[
                    "-d",
                    db_path,
                    "-c",
                    "-r",
                    r"module_[0-9]+\.rs",
                ]));
            });
        });

        group.finish();

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&db);
    }

    criterion_group!(benches, bench_e2e);
}

#[cfg(unix)]
criterion::criterion_main!(bench::benches);

#[cfg(not(unix))]
fn main() {}
