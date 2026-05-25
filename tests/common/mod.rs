//! Serialize nested `cargo` subprocesses across integration test binaries.
//!
//! Parallel integration tests each spawning `cargo build` / `cargo run` can fill `/tmp` and
//! overwhelm small disks; an advisory lock keeps one nested build at a time.
//!
//! `flock` alone is not enough: on common Unix semantics, locks taken from multiple open file
//! descriptions in the **same process** do not block each other, so parallel threads in one
//! integration test crate would still run nested `cargo` concurrently. We take an in-process mutex
//! first, then an `flock` so different test *processes* also serialize.

use fs4::fs_std::FileExt;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

static IN_PROCESS: Mutex<()> = Mutex::new(());

/// Held until dropped; releases the lock when the underlying file is closed.
pub struct NestedCargoLockGuard {
    /// Struct fields drop in reverse order: `_file` first (end cross-process `flock`), then this
    /// guard (end in-process mutex).
    _in_process: MutexGuard,
    _file: std::fs::File,
}

type MutexGuard = std::sync::MutexGuard<'static, ()>;

/// Shared `target/` for nested `cargo build` / `cargo run` in generated projects.
///
/// Without this, each temp project compiles heavy deps (e.g. `aws-lc-sys`) into `/tmp` and can
/// exhaust disk on developer machines and CI runners.
#[allow(dead_code)]
pub fn nested_cargo_target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("nested-integration")
}

/// `cargo` for building/running generated integration-test projects.
#[allow(dead_code)]
pub fn nested_project_cargo() -> io::Result<Command> {
    let target_dir = nested_cargo_target_dir();
    fs::create_dir_all(&target_dir)?;
    let mut cmd = Command::new("cargo");
    cmd.env("CARGO_TARGET_DIR", target_dir);
    Ok(cmd)
}

/// Blocks until no other integration test holds the nested-cargo lock.
pub fn nested_cargo_lock() -> io::Result<NestedCargoLockGuard> {
    let in_process = IN_PROCESS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = std::env::temp_dir().join("corvo_lang_nested_cargo_integration.lock");
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    f.lock_exclusive()?;
    Ok(NestedCargoLockGuard {
        _in_process: in_process,
        _file: f,
    })
}
