//! Serialize nested `cargo` subprocesses across integration test binaries.
//!
//! Parallel integration tests each spawning `cargo build` / `cargo run` can fill `/tmp` and
//! overwhelm small disks; an advisory lock keeps one nested build at a time.

use fs4::fs_std::FileExt;
use std::fs::OpenOptions;
use std::io;

/// Held until dropped; releases the lock when the underlying file is closed.
pub struct NestedCargoLockGuard(std::fs::File);

/// Blocks until no other integration test holds the nested-cargo lock.
pub fn nested_cargo_lock() -> io::Result<NestedCargoLockGuard> {
    let path = std::env::temp_dir().join("corvo_lang_nested_cargo_integration.lock");
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)?;
    f.lock_exclusive()?;
    Ok(NestedCargoLockGuard(f))
}
