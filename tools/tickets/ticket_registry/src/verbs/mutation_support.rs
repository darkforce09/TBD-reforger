//! The steps every writing verb shares.
//!
//! **Role:** load the typed corpus, reload the registry value after a write, refresh the wave
//! lock after a status write, and wrap an operation's refusal as [`Error::Refused`].
//! **Position:** private helpers of the verbs module.
//! **Signals & state:** none; reads the ticket files and writes the wave lock.
//! **Invariants:** every step after a write reads the post-write files, never the value the verb
//! started with.

use super::*;
use crate::error::{Error, Result, ResultExt};

/// Lifecycle hook: every registry STATUS writer refreshes the committed wave.lock with
/// the one legal writer, so a bookkeeping ship/cancel never leaves `wave check` red on a
/// correct registry. The refresh rides whatever commit carries the status change — statuses and
/// the lock are working-tree writes the operator commits together.
pub(super) fn refresh_wave_lock(root: &Path) -> Result<()> {
    ticket_wave_lock::repack_quiet(root)
        .map(|_| ())
        .context("refresh wave.lock after status write (`cargo xtask wave repack`)")
}

/// Typed corpus load for the mutators. Fail-closed like [`ticket_model::Corpus::load`]: one
/// unparseable ticket file refuses the whole load, naming the file. The full corpus (parents
/// AND children) is what makes dotted child ids resolve; the parents-only `require_ticket`
/// view would answer "Unknown ticket" for every dotted child id.
pub(super) fn load_corpus(root: &Path) -> Result<Corpus> {
    Corpus::load(root).map_err(Error::msg)
}

/// The typed operations' refusals (mark-ready's spec and dependency gates, reorder's anchor,
/// advance-slice's slice walk) as [`Error::Refused`]: the command prints the message bare on
/// stderr and exits 1, with no `xtask:` prefix.
pub(super) fn refuse_verbatim(message: String) -> Error {
    Error::Refused { message }
}

/// Re-reads the registry `Value` from disk after a typed op writes: the reload-before-sync
/// invariant. By the time any post-write step runs, the typed op has
/// ALREADY landed its files; the `Value` those steps consume MUST be re-read from disk.
/// Passing the pre-mutation Value to `cmd_sync` / `generate_queue_json` regenerates queue.json,
/// the roadmap next-work block and the gap-analysis ticket column from the OLD state — pinned
/// by `ship_regenerates_queue_from_post_state_reload_pin`. The reload is also what surfaces
/// a typed CHILD write into the parents-only Value view: `attach_slice_plan` re-synthesizes
/// `slice_plan` from the child files.
pub(super) fn reload_registry(root: &Path, registry: &mut Value) -> Result<()> {
    *registry = crate::registry::typed_projection::load_phase2_tree(root)?;
    Ok(())
}
