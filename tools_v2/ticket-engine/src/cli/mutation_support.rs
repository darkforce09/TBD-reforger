//! Mutation support.

use super::*;
use anyhow::Context;

/// T-912.2 lifecycle hook: every registry STATUS writer refreshes the committed wave.lock with
/// the one legal writer, so a bookkeeping ship/cancel never leaves `wave check` red on a
/// correct registry. The refresh rides whatever commit carries the status change — statuses and
/// the lock are working-tree writes the operator commits together.
pub(super) fn refresh_wave_lock(root: &Path) -> Result<()> {
    crate::wave_lock::repack_quiet(root)
        .map(|_| ())
        .context("refresh wave.lock after status write (`cargo xtask wave repack`)")
}

/// T-916.2 — typed corpus load for the mutators. Fail-closed like [`crate::Corpus::load`]: one
/// unparseable ticket file refuses the whole load, naming the file. The full corpus (parents
/// AND children) is what makes dotted child ids resolve — the parents-only `require_ticket`
/// view was the "`ticket ship T-912.2` → Unknown ticket" hole.
pub(super) fn load_corpus(root: &Path) -> Result<Corpus> {
    Corpus::load(root).map_err(anyhow::Error::msg)
}

/// T-916.2 — refusals the pre-typed mutators printed BARE on stderr + exit 1 (mark-ready's
/// spec/deps gates, reorder's anchor, advance-slice's slice walk). The typed ops return the
/// same strings as `Err`; this shim keeps the exit shape byte-identical for external callers
/// instead of adding anyhow's `xtask:` prefix.
pub(super) fn refuse_verbatim(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

/// T-916.2 — the reload-before-sync invariant (t915_ticketboard_design.md §Write path,
/// "Rewiring sequence invariant"). By the time any post-write step runs, the typed op has
/// ALREADY landed its files; the `Value` those steps consume MUST be re-read from disk.
/// Passing the pre-mutation Value to `cmd_sync` / `generate_queue_json` regenerates queue.json
/// and every generated doc from the OLD state — pinned by
/// `ship_regenerates_docs_from_post_state_reload_pin` below. The reload is also what surfaces
/// a typed CHILD write into the parents-only Value view: `attach_slice_plan` re-synthesizes
/// `slice_plan` from the child files.
pub(super) fn reload_registry(root: &Path, registry: &mut Value) -> Result<()> {
    *registry = crate::registry::typed_projection::load_phase2_tree(root)?;
    Ok(())
}
