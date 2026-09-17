use super::*;

/// The behaviour half of the T-779 pin — [`commit_one_write`] driven natively with a
/// refusing sink, the exact production shape for an unknown id. The wiring half (the
/// live `editor_ops` and panel scrub pins) stays in `arsenal/mod.rs::tests::t779`.
mod t779 {
    use super::*;

    /// **The acceptance test: a write the document REFUSES mints no history tail.**
    ///
    /// `update_slot_loadout` returns `false` for an id the document does not hold — an entity
    /// deleted, or undone away, while the Arsenal sat open over it. A test that only ever used
    /// a valid id could not observe this defect at all: with the hardcoded `true` in place, the
    /// valid-id path behaved identically before and after the fix.
    #[test]
    fn a_refused_write_mints_no_tail_and_does_not_dirty_the_mission() {
        // The refusal. `tails` stands in for `mission_history::after_local_edit` — which both
        // sets `HistoryCtx::dirty` and mints the undo step, so one counter answers both halves
        // of the acceptance: no tail fired means nothing was dirtied and no step was minted.
        let mut tails = 0usize;
        let took = commit_one_write(|| false, || tails += 1);
        assert!(
            !took,
            "T-779: a refused write must report itself refused, not report success"
        );
        assert_eq!(
            tails, 0,
            "T-779: a refused write must mint no history tail — the document did not change, \
             so there is nothing to dirty and nothing for Ctrl+Z to restore"
        );

        // The accepted write, so the pin cannot pass by never firing the tail at all.
        let mut tails = 0usize;
        let took = commit_one_write(|| true, || tails += 1);
        assert!(took, "T-779: an acknowledged write must report success");
        assert_eq!(
            tails, 1,
            "T-779: an acknowledged write is exactly one tail — one undo step per pick (T-732)"
        );

        // The gate must read the SINK, not the fact that a commit closure ran. Counting
        // invocations is precisely the T-770 defect, one layer down.
        let mut ran = 0usize;
        let mut tails = 0usize;
        let took = commit_one_write(
            || {
                ran += 1;
                false
            },
            || tails += 1,
        );
        assert_eq!(ran, 1, "the commit closure must still be called");
        assert!(!took);
        assert_eq!(
            tails, 0,
            "T-779: the tail is gated on the ACK, not on the closure having been invoked"
        );
    }
}
