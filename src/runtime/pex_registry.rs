use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub const KIND_SESSION: &str = "pex_session";
pub const KIND_BASH: &str = "pex_bash";

/// Live PTY sessions for `pex.*` builtins. Uses interior mutability so `standard_lib::call`
/// can stay `&RuntimeState`.
#[derive(Default)]
pub struct PexRegistry {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    entries: Mutex<HashMap<u64, PexEntry>>,
    #[cfg(all(unix, feature = "stdlib-pex"))]
    next_id: AtomicU64,
}

impl std::fmt::Debug for PexRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PexRegistry")
    }
}

#[cfg(all(unix, feature = "stdlib-pex"))]
pub enum PexEntry {
    Session(rexpect::session::PtySession),
    Repl(rexpect::session::PtyReplSession),
}

impl PexRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(all(unix, feature = "stdlib-pex"))]
    pub fn insert_session(&self, session: rexpect::session::PtySession) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.entries
            .lock()
            .unwrap()
            .insert(id, PexEntry::Session(session));
        id
    }

    #[cfg(all(unix, feature = "stdlib-pex"))]
    pub fn insert_repl(&self, repl: rexpect::session::PtyReplSession) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.entries
            .lock()
            .unwrap()
            .insert(id, PexEntry::Repl(repl));
        id
    }

    #[cfg(all(unix, feature = "stdlib-pex"))]
    pub fn with_entry_mut<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut PexEntry) -> CorvoResult<R>,
    ) -> CorvoResult<R> {
        let mut guard = self.entries.lock().unwrap();
        let entry = guard
            .get_mut(&id)
            .ok_or_else(|| CorvoError::invalid_argument("pex: unknown or closed session handle"))?;
        f(entry)
    }

    #[cfg(all(unix, feature = "stdlib-pex"))]
    pub fn remove(&self, id: u64) -> CorvoResult<()> {
        self.entries
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or_else(|| CorvoError::invalid_argument("pex.close: unknown session handle"))?;
        Ok(())
    }
}
