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
use std::fs::OpenOptions;
use std::io;
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

/// Blocks until no other integration test holds the nested-cargo lock.
pub fn nested_cargo_lock() -> io::Result<NestedCargoLockGuard> {
    let in_process = IN_PROCESS
        .lock()
        .map_err(|_| io::Error::other("nested cargo in-process mutex poisoned"))?;
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
