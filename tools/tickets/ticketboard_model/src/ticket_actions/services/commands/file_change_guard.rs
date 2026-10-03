//! The compare-and-swap guard on a ticket file.
//!
//! **Role:** `FileChangeGuard`, `fingerprint_file`, `guard_for` and `cas_ok`.
//! **Position:** part of `crate::ticket_actions::services::commands`; the dialogs take a guard, and
//! the desktop application checks `cas_ok` before every dispatch.
//! **Signals & state:** none; reads the ticket file.
//! **Invariants:** a command runs only while the file still matches the fingerprint taken when the
//! action was offered.

use super::*;
// ---- compare-and-swap guard ----

/// Fingerprint of the target ticket file, captured when the affordance was
/// rendered/clicked. At dispatch the file is re-hashed; a mismatch refuses the
/// dispatch (no subprocess) with a "file changed on disk — reloading" toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeGuard {
    /// The ticket file the command targets.
    pub path: PathBuf,
    /// FNV-1a over the file bytes; `None` = unreadable/absent at capture time.
    pub pre: Option<u64>,
}

/// FNV-1a 64 — cheap, dependency-free, and content-based on purpose (mtime lies
/// under editors that preserve timestamps; length misses same-length edits).
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The FNV-1a fingerprint of the file at `path`, `None` when it cannot be read.
pub fn fingerprint_file(path: &Path) -> Option<u64> {
    fs::read(path).ok().map(|bytes| hash_bytes(&bytes))
}

/// Capture the guard for a ticket file NOW (render-of-menu / click time).
pub fn guard_for(path: &Path) -> FileChangeGuard {
    FileChangeGuard {
        path: path.to_path_buf(),
        pre: fingerprint_file(path),
    }
}

/// Re-hash at dispatch time. `None` (no target file — `add`) always passes; a
/// guard passes only when the bytes hash identically, including the
/// both-absent case (still-missing file: the verb's own "Unknown ticket"
/// refusal is the right surface, not a CAS refusal).
pub fn cas_ok(guard: Option<&FileChangeGuard>) -> bool {
    match guard {
        None => true,
        Some(g) => fingerprint_file(&g.path) == g.pre,
    }
}
