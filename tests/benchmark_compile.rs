//! Compile and transpile benchmarks over `examples/*.corvo`.
//!
//! Run (from the repo root; may take a long time):
//!
//! ```text
//! CORVO_LANG_LOCAL_PATH="$(pwd)" cargo test --test benchmark_compile generate_benchmark_report -- --ignored --nocapture
//! ```
//!
//! Optional:
//!
//! - `CORVO_BENCHMARK_LIMIT` — max number of example files (sorted by path). Default: all.
//! - `CORVO_BENCHMARK_SKIP_PER_FILE_COLD` — if `1`, skip the per-example cold-cache phase
//!   (each example uses a fresh `CORVO_CACHE_DIR`; very expensive for large sets).

#[path = "common/mod.rs"]
mod common;

use corvo_lang::compiler::builder::corvo_cache_dir_test_lock;
use corvo_lang::compiler::{append_corvo_lang_patch_to_cargo_toml, Compiler};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const CACHE_ENV: &str = "CORVO_CACHE_DIR";

struct EnvRestore {
    key: &'static str,
    old: Option<OsString>,
}

impl EnvRestore {
    fn set(key: &'static str, value: &Path) -> Self {
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match &self.old {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn examples_dir() -> PathBuf {
    repo_root().join("examples")
}

fn list_example_files() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(examples_dir())
        .expect("read examples/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "corvo"))
        .collect();
    paths.sort();
    // Prefer `hello.corvo` first so cold/warm baselines use a tiny script (faster, fewer surprises).
    if let Some(i) = paths
        .iter()
        .position(|p| p.file_name().and_then(|n| n.to_str()) == Some("hello.corvo"))
    {
        let hello = paths.remove(i);
        paths.insert(0, hello);
    }
    let limit = std::env::var("CORVO_BENCHMARK_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    paths.truncate(limit.min(paths.len()));
    paths
}

fn binary_output_path(out_dir: &Path, stem: &str) -> PathBuf {
    let name = if cfg!(target_os = "windows") {
        format!("{}.exe", stem)
    } else {
        stem.to_string()
    };
    out_dir.join(name)
}

fn file_len(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|m| m.len())
}

fn dur_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn compile_corvo_to(
    script: &Path,
    out_bin: &Path,
) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
    let source = fs::read_to_string(script)?;
    let mut compiler = Compiler::new(source, script.to_path_buf());
    compiler
        .pre_execute()
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
    let t0 = Instant::now();
    let _final = compiler.compile(out_bin)?;
    Ok(t0.elapsed())
}

fn append_patch_crates_io(project_dir: &Path, crate_root: &Path) -> Result<(), String> {
    let manifest = project_dir.join("Cargo.toml");
    let canon = crate_root
        .canonicalize()
        .map_err(|e| format!("canonicalize {}: {}", crate_root.display(), e))?;
    append_corvo_lang_patch_to_cargo_toml(&manifest, &canon).map_err(|e| e.to_string())
}

fn cargo_build_release(project_dir: &Path) -> Result<Duration, String> {
    let _nested_cargo = common::nested_cargo_lock().map_err(|e| e.to_string())?;
    let t0 = Instant::now();
    let out = Command::new("cargo")
        .args(["build", "--release"])
        .env_remove("CARGO_TARGET_DIR")
        .current_dir(project_dir)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "cargo build failed in {}:\n{}",
            project_dir.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(t0.elapsed())
}

fn transpile_example(script: &Path, project_dir: &Path) -> Result<Duration, String> {
    let source = fs::read_to_string(script).map_err(|e| e.to_string())?;
    let mut compiler = Compiler::new(source, script.to_path_buf());
    compiler.pre_execute().map_err(|e| e.to_string())?;
    let t0 = Instant::now();
    compiler.transpile(project_dir).map_err(|e| e.to_string())?;
    Ok(t0.elapsed())
}

fn benchmark_hostname() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        let t = h.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        let t = h.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Ok(out) = Command::new("hostname").output() {
        if out.status.success() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                let t = s.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    #[cfg(unix)]
    if let Ok(s) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    "unknown".to_string()
}

fn rust_version_line() -> String {
    Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "rustc (unknown)".to_string())
}

#[test]
fn benchmark_examples_dir_exists() {
    assert!(examples_dir().is_dir(), "examples/ missing");
    assert!(
        !list_example_files().is_empty(),
        "no .corvo files under examples/"
    );
}

/// Regenerates `BENCHMARK.md` in the repo root. **Ignored** by default (slow; runs `cargo build`).
#[ignore]
#[test]
fn generate_benchmark_report() {
    let _global_cache_lock = corvo_cache_dir_test_lock();

    let examples = list_example_files();
    assert!(
        !examples.is_empty(),
        "no examples to benchmark (check CORVO_BENCHMARK_LIMIT)"
    );

    let root = repo_root();
    let _patch_guard = EnvRestore::set("CORVO_LANG_LOCAL_PATH", &root);

    let out_root = tempfile::tempdir().expect("tempdir");
    let compile_out = out_root.path().join("compiled");
    fs::create_dir_all(&compile_out).unwrap();
    let skip_per_file_cold = std::env::var("CORVO_BENCHMARK_SKIP_PER_FILE_COLD")
        .map(|s| s == "1")
        .unwrap_or(false);

    let mut md = String::new();
    md.push_str("# Corvo compile & transpile benchmarks\n\n");
    md.push_str("This file is generated by `tests/benchmark_compile.rs`. Do not edit by hand.\n\n");
    md.push_str("```text\nCORVO_LANG_LOCAL_PATH=\"$(pwd)\" \\\n");
    md.push_str(
        "  cargo test --test benchmark_compile generate_benchmark_report -- --ignored --nocapture\n```\n\n",
    );

    let hostname = benchmark_hostname();
    md.push_str("## Environment\n\n");
    md.push_str(&format!(
        "- **Timestamp** : {:?}\n",
        std::time::SystemTime::now()
    ));
    md.push_str(&format!("- **Host**       : {}\n", hostname));
    md.push_str(&format!("- **Rust**       : {}\n", rust_version_line()));
    md.push_str(&format!("- **Examples**   : {} file(s)", examples.len()));
    if std::env::var("CORVO_BENCHMARK_LIMIT").is_ok() {
        md.push_str(" (`CORVO_BENCHMARK_LIMIT` set)");
    }
    md.push_str("\n\n---\n\n");

    // Shared compile cache for cold → warm → all-examples-shared
    let shared_cache = tempfile::tempdir().unwrap();
    let _shared_cache_guard = EnvRestore::set(CACHE_ENV, shared_cache.path());

    let first = examples.first().unwrap();
    let cold_bin = binary_output_path(&compile_out, first.file_stem().unwrap().to_str().unwrap());
    md.push_str("## `corvo --compile` (`Compiler::compile`, release)\n\n");
    md.push_str("### Cold cache (fresh `CORVO_CACHE_DIR`, first example)\n\n");
    md.push_str("| Metric | Value |\n|--------|-------|\n");
    md.push_str(&format!(
        "| Example | `{}` |\n",
        first.strip_prefix(&root).unwrap_or(first).display()
    ));
    match compile_corvo_to(first, &cold_bin) {
        Ok(t_cold) => {
            md.push_str(&format!(
                "| Wall time (`compile` only; includes `cargo build`) | {:.2} ms |\n",
                dur_ms(t_cold)
            ));
            md.push_str(&format!(
                "| Output binary size | {} bytes |\n\n",
                file_len(&cold_bin).unwrap_or(0)
            ));
        }
        Err(e) => {
            md.push_str(&format!(
                "| Wall time (`compile` only; includes `cargo build`) | ERROR: {} |\n",
                e
            ));
            md.push_str("| Output binary size | — |\n\n");
        }
    }

    if examples.len() >= 2 {
        let second = &examples[1];
        let warm_bin =
            binary_output_path(&compile_out, second.file_stem().unwrap().to_str().unwrap());
        md.push_str("### Warm cache (same `CORVO_CACHE_DIR`, second example)\n\n");
        md.push_str("| Metric | Value |\n|--------|-------|\n");
        md.push_str(&format!(
            "| Example | `{}` |\n",
            second.strip_prefix(&root).unwrap_or(second).display()
        ));
        match compile_corvo_to(second, &warm_bin) {
            Ok(t_warm) => {
                md.push_str(&format!("| Wall time | {:.2} ms |\n", dur_ms(t_warm)));
                md.push_str(&format!(
                    "| Output binary size | {} bytes |\n\n",
                    file_len(&warm_bin).unwrap_or(0)
                ));
            }
            Err(e) => {
                md.push_str(&format!("| Wall time | ERROR: {} |\n", e));
                md.push_str("| Output binary size | — |\n\n");
            }
        }
    }

    md.push_str("### Shared cache — all listed examples (sequential)\n\n");
    md.push_str(
        "| Example | Time (ms) | Binary size (bytes) |\n|---------|-----------|---------------------|\n",
    );
    let mut total_shared = Duration::ZERO;
    let mut shared_ok: usize = 0;
    for p in &examples {
        let stem = p.file_stem().unwrap().to_str().unwrap();
        let bin = binary_output_path(&compile_out, stem);
        let label = p.strip_prefix(&root).unwrap_or(p).display().to_string();
        match compile_corvo_to(p, &bin) {
            Ok(d) => {
                total_shared += d;
                shared_ok += 1;
                let sz = file_len(&bin).unwrap_or(0);
                md.push_str(&format!("| `{}` | {:.2} | {} |\n", label, dur_ms(d), sz));
            }
            Err(e) => {
                md.push_str(&format!("| `{}` | ERROR | {} |\n", label, e));
            }
        }
    }
    md.push_str(&format!(
        "\n**Total wall time (compile steps, sequential)** : {:.2} ms  \n**Succeeded** : {} / {}\n\n---\n\n",
        dur_ms(total_shared),
        shared_ok,
        examples.len()
    ));

    md.push_str("## `corvo --compile` — fresh `CORVO_CACHE_DIR` per example (cold each time)\n\n");
    if skip_per_file_cold {
        md.push_str("*Skipped (`CORVO_BENCHMARK_SKIP_PER_FILE_COLD=1`).*\n\n---\n\n");
    } else {
        md.push_str(
            "| Example | Time (ms) | Binary size (bytes) |\n|---------|-----------|---------------------|\n",
        );
        let mut total_cold_all = Duration::ZERO;
        let mut cold_ok = 0usize;
        for p in &examples {
            let isolate = tempfile::tempdir().unwrap();
            let _g = EnvRestore::set(CACHE_ENV, isolate.path());
            let stem = p.file_stem().unwrap().to_str().unwrap();
            let bin = binary_output_path(&compile_out, format!("cold_{}", stem).as_str());
            let label = p.strip_prefix(&root).unwrap_or(p).display().to_string();
            match compile_corvo_to(p, &bin) {
                Ok(d) => {
                    total_cold_all += d;
                    cold_ok += 1;
                    let sz = file_len(&bin).unwrap_or(0);
                    md.push_str(&format!("| `{}` | {:.2} | {} |\n", label, dur_ms(d), sz));
                }
                Err(e) => {
                    md.push_str(&format!("| `{}` | ERROR | {} |\n", label, e));
                }
            }
        }
        md.push_str(&format!(
            "\n**Total wall time** : {:.2} ms  \n**Succeeded** : {} / {}\n\n---\n\n",
            dur_ms(total_cold_all),
            cold_ok,
            examples.len()
        ));
    }

    drop(_shared_cache_guard);

    md.push_str("## Transpile → one Cargo project → `cargo build --release`\n\n");
    let single = tempfile::tempdir().unwrap();
    let t_tr_wall = Instant::now();
    let mut transpile_sum = Duration::ZERO;
    let mut tr_ok = 0usize;
    for p in &examples {
        match transpile_example(p, single.path()) {
            Ok(d) => {
                transpile_sum += d;
                tr_ok += 1;
            }
            Err(e) => eprintln!("transpile skip {}: {}", p.display(), e),
        }
    }
    let tr_wall = t_tr_wall.elapsed();
    if tr_ok == 0 {
        md.push_str("*No examples transpiled; skipping cargo build.*\n\n---\n\n");
    } else {
        append_patch_crates_io(single.path(), &root).expect("patch single");
        let t_cb = cargo_build_release(single.path()).expect("cargo single");
        md.push_str("| Phase | Time (ms) |\n|-------|----------|\n");
        md.push_str(&format!(
            "| Transpile (sum of per-file inner timers) | {:.2} |\n",
            dur_ms(transpile_sum)
        ));
        md.push_str(&format!(
            "| Transpile (wall clock) | {:.2} |\n",
            dur_ms(tr_wall)
        ));
        md.push_str(&format!(
            "| `cargo build --release` (all binaries) | {:.2} |\n",
            dur_ms(t_cb)
        ));
        md.push_str(&format!(
            "| **Total** | {:.2} |\n\n",
            dur_ms(tr_wall + t_cb)
        ));
        md.push_str("### Binary sizes (`target/release/`)\n\n");
        md.push_str("| Binary | Size (bytes) |\n|--------|--------------|\n");
        let tr = single.path().join("target/release");
        for p in &examples {
            let stem = p.file_stem().unwrap().to_str().unwrap();
            let name = if cfg!(target_os = "windows") {
                format!("{}.exe", stem)
            } else {
                stem.to_string()
            };
            let bp = tr.join(&name);
            if let Some(sz) = file_len(&bp) {
                md.push_str(&format!("| `{}` | {} |\n", name, sz));
            }
        }
        md.push_str(&format!(
            "\n*Transpiled: {} / {}*\n\n---\n\n",
            tr_ok,
            examples.len()
        ));
    }

    md.push_str("## Transpile → separate Cargo project per example → `cargo build --release`\n\n");
    md.push_str(
        "| Example | Transpile (ms) | Cargo build (ms) | Binary size (bytes) |\n|---------|----------------|-----------------|---------------------|\n",
    );
    let mut multi_total_tr = Duration::ZERO;
    let mut multi_total_cb = Duration::ZERO;
    for p in &examples {
        let label = p.strip_prefix(&root).unwrap_or(p).display().to_string();
        let proj = tempfile::tempdir().unwrap();
        let stem = p.file_stem().unwrap().to_str().unwrap();
        match transpile_example(p, proj.path()) {
            Ok(d_tr) => {
                multi_total_tr += d_tr;
                if let Err(e) = append_patch_crates_io(proj.path(), &root) {
                    md.push_str(&format!("| `{}` | ERROR | patch: {} | |\n", label, e));
                    continue;
                }
                match cargo_build_release(proj.path()) {
                    Ok(d_cb) => {
                        multi_total_cb += d_cb;
                        let name = if cfg!(target_os = "windows") {
                            format!("{}.exe", stem)
                        } else {
                            stem.to_string()
                        };
                        let bp = proj.path().join("target/release").join(&name);
                        let sz = file_len(&bp).unwrap_or(0);
                        md.push_str(&format!(
                            "| `{}` | {:.2} | {:.2} | {} |\n",
                            label,
                            dur_ms(d_tr),
                            dur_ms(d_cb),
                            sz
                        ));
                    }
                    Err(e) => md.push_str(&format!(
                        "| `{}` | {:.2} | ERROR | {} |\n",
                        label,
                        dur_ms(d_tr),
                        e
                    )),
                }
            }
            Err(e) => {
                md.push_str(&format!("| `{}` | ERROR | | {} |\n", label, e));
            }
        }
    }
    md.push_str(&format!(
        "\n**Sum transpile** : {:.2} ms  \n**Sum `cargo build`** : {:.2} ms  \n**Sum both** : {:.2} ms\n",
        dur_ms(multi_total_tr),
        dur_ms(multi_total_cb),
        dur_ms(multi_total_tr + multi_total_cb)
    ));

    let dest = root.join("BENCHMARK.md");
    let mut f = fs::File::create(&dest).expect("BENCHMARK.md");
    f.write_all(md.as_bytes()).expect("write BENCHMARK.md");
    eprintln!("Wrote {}", dest.display());
}
