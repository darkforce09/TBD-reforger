//! T-190 (F-32) — two tabs on one mission.
//!
//! **Defect-proof commit.** This file currently holds nothing but the pins that state what
//! `persist.rs` / `hydrate.rs` / `canvas/overlays.rs` must do and today do not. The cross-tab
//! presence machinery lands in the next commit, on top of a RED that was recorded first.

#[cfg(test)]
mod tests {
    use crate::editor::arsenal::class_r_scrub::{live_code, live_source, only_item};

    fn persist_live() -> String {
        live_code(include_str!("persist.rs"))
    }

    fn persist_src() -> String {
        live_source(include_str!("persist.rs"))
    }

    fn overlays_src() -> String {
        live_source(include_str!("../canvas/overlays.rs"))
    }

    /// RED on the pre-T-190 tree: `run_save` reads NOTHING before it writes. Its four guards
    /// (`is_cancelled`, the T-221 owner token, the T-374 content probe, the T-374 unreadable
    /// record) all answer questions about the *incoming* blob, and then `save_state_as` puts the
    /// whole key. A sibling tab's bytes at that key are overwritten unseen — F-32's verified repro.
    #[test]
    fn t190_run_save_merges_the_stored_record_before_it_writes() {
        let run = only_item(&persist_live(), "async fn run_save(").to_string();
        assert!(
            run.contains("read_raw") || run.contains("load_state"),
            "run_save must READ the record it is about to overwrite. run={run}"
        );
        assert!(
            run.contains("apply_update") || run.contains("merge_stored"),
            "the stored blob must be applied as a yrs update, never diffed or discarded. run={run}"
        );
    }

    /// RED: nothing in the write path knows another tab exists, so the second tab's debounce
    /// simply wins. The banner is the visible half; this is the half that keeps the bytes safe.
    #[test]
    fn t190_a_second_tab_cannot_silently_overwrite_the_first() {
        let live = persist_live();
        let run = only_item(&live, "async fn run_save(").to_string();
        assert!(
            run.contains("tab_lock") || run.contains("may_write"),
            "run_save must consult the cross-tab role before writing. run={run}"
        );
    }

    /// RED: `ConflictInfo` carries exactly two fields — `payload_json` and `semver` — so the
    /// dialog cannot name what either option contains or when it was written.
    #[test]
    fn t190_conflict_info_carries_counts_and_timestamps() {
        let info = only_item(&overlays_src(), "pub struct ConflictInfo").to_string();
        for field in [
            "local_objects",
            "server_objects",
            "local_saved",
            "server_saved",
        ] {
            assert!(
                info.contains(field),
                "ConflictInfo must describe BOTH options; missing `{field}`. info={info}"
            );
        }
    }

    /// RED: "Load server version" is the AFFIRMATIVE button (`bg-primary`), and no copy anywhere
    /// in the dialog says it destroys the local document. F-32's first complaint.
    #[test]
    fn t190_load_server_version_is_marked_destructive() {
        let dialog = only_item(&overlays_src(), "pub(crate) fn ConflictDialog(").to_string();
        let at = dialog
            .find("Load server version")
            .unwrap_or_else(|| panic!("the dialog must still offer the server version. {dialog}"));
        let arm = &dialog[..at];
        let arm = &arm[arm.rfind("<button").unwrap_or(0)..];
        assert!(
            arm.contains("error"),
            "the destructive choice must use the codebase's destructive tokens \
             (text-error / bg-error), not bg-primary. arm={arm}"
        );
        assert!(
            dialog.contains("discard") || dialog.contains("Discard"),
            "the dialog must SAY that loading the server version discards local work. {dialog}"
        );
    }

    /// T-946.51 — RED: `SAVE_IN_FLIGHT` is `.set(true)` in `run_save` and `.set(false)` in
    /// `SaveFlightGuard::drop`, and read by nothing. Two comments (`run_save`'s thread_local doc
    /// and `register_flush_on_hide`'s pagehide note) claim it serialises the hidden flush against
    /// pagehide; the per-mission `lock_for(id)` mutex is what actually does that.
    #[test]
    fn t946_51_save_in_flight_is_read_by_something() {
        let live = persist_live();
        let reads = live.match_indices("SAVE_IN_FLIGHT").filter(|(i, _)| {
            let tail = &live[*i..];
            !tail.starts_with("SAVE_IN_FLIGHT.set(")
                && !tail.starts_with("SAVE_IN_FLIGHT: Cell<bool>")
                && !tail.starts_with("SAVE_IN_FLIGHT: Cell < bool >")
        });
        let n = reads.count();
        assert!(
            n > 0,
            "SAVE_IN_FLIGHT is written and never read — either make it load-bearing or delete it \
             (T-946.51). Occurrences that are not a `.set(` or the declaration: {n}"
        );
        let src = persist_src();
        assert!(
            !src.contains("serialized by SAVE_IN_FLIGHT"),
            "the pagehide comment must not credit SAVE_IN_FLIGHT with serialisation that \
             `lock_for(id)` performs (T-946.51)"
        );
    }

    /// T-946.54 — RED: `note_unreadable` sleeps 80 / 160 / 320 ms and only THEN inserts the key
    /// into `UNREADABLE`. For those ~560 ms `run_save`'s fourth guard sees an empty set, so a
    /// record this session has already failed to read can be overwritten. Latch first, retry after.
    #[test]
    fn t946_54_note_unreadable_latches_before_it_retries() {
        let item = only_item(&persist_live(), "async fn note_unreadable(").to_string();
        let latch = item
            .find("UNREADABLE.with")
            .unwrap_or_else(|| panic!("note_unreadable must touch UNREADABLE. item={item}"));
        let first_sleep = item
            .find("sleep_ms")
            .unwrap_or_else(|| panic!("note_unreadable must back off. item={item}"));
        assert!(
            latch < first_sleep,
            "the key must be latched into UNREADABLE BEFORE the first backoff, or run_save passes \
             the T-374 guard for the whole retry window (T-946.54). item={item}"
        );
    }
}
