//! Tab Lock tests tests.

use super::*;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_item};

fn persist_text() -> String {
    [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/shell/persist.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/shell/persist/record_store.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/shell/persist/save_scheduler.rs"
        )),
    ]
    .join("\n")
}

fn persist_live() -> String {
    live_code(&persist_text())
}

fn persist_src() -> String {
    live_source(&persist_text())
}

fn overlays_src() -> String {
    live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/overlays/conflict_dialog.rs"
    )))
}

fn p(tab: &str, since: f64) -> Presence {
    let tab = tab.to_string();
    Presence { tab, since }
}

fn st(tab: &str, at: f64) -> Stamp {
    let tab = tab.to_string();
    Stamp { tab, at }
}

/* ── the policy, behaviourally ── */

/// The F-32 repro at the level this crate can execute on the host: two tabs, one key. Before
/// T-190 every save was an unconditional write-through, so B's debounce landed on top of A's
/// bytes having read nothing — "the last debounce wins". The policy now answers `Merge` for
/// exactly the case that produced the loss (a record stamped by another tab) and `Defer` for
/// the tab that should not be writing at all.
#[test]
fn t190_a_foreign_record_is_merged_not_overwritten() {
    let (mine, theirs) = (st("tab-a", 10.0), st("tab-b", 20.0));
    assert_eq!(
        decide_save(TabRole::Writer, Some(&theirs), "tab-a"),
        SaveDecision::Merge,
        "a record another tab wrote must be read and merged, never overwritten blind"
    );
    assert_eq!(
        decide_save(TabRole::Writer, None, "tab-a"),
        SaveDecision::Merge,
        "an unattributable record is the one that may be somebody's only copy (T-221 posture)"
    );
    assert_eq!(
        decide_save(TabRole::Writer, Some(&mine), "tab-a"),
        SaveDecision::WriteThrough,
        "a record this tab wrote is a subset of its own document — no decode needed"
    );
}

#[test]
fn t190_a_read_only_tab_defers_instead_of_writing_or_dropping() {
    let theirs = st("tab-b", 20.0);
    for stamp in [None, Some(&theirs)] {
        assert_eq!(
            decide_save(TabRole::ReadOnly, stamp, "tab-a"),
            SaveDecision::Defer,
            "the second tab must not write — and must not silently drop the pending either"
        );
    }
}

#[test]
fn t190_the_oldest_tab_writes_and_the_election_is_total() {
    let (a, b) = (p("tab-a", 100.0), p("tab-b", 200.0));
    let ab = [a.clone(), b.clone()];
    assert_eq!(elect(&a, &ab), TabRole::Writer);
    assert_eq!(elect(&b, &ab), TabRole::ReadOnly);
    assert_eq!(elect(&a, &[]), TabRole::Writer, "alone ⇒ writer");
    // A tie on the instant must still elect exactly one, or both tabs write.
    let (x, y) = (p("aaa", 100.0), p("bbb", 100.0));
    let both = [x.clone(), y.clone()];
    assert_eq!(elect(&x, &both), TabRole::Writer);
    assert_eq!(elect(&y, &both), TabRole::ReadOnly);
}

#[test]
fn t190_stamp_round_trips_and_the_key_is_namespaced() {
    let s = st("tab-a", 1_725_000_000_000.0);
    let text = serde_json::to_string(&s).expect("stamp serialises");
    assert_eq!(serde_json::from_str::<Stamp>(&text).expect("round trip"), s);
    let key = stamp_key("u4:1234|mission-9");
    assert!(key.starts_with(STAMP_PREFIX) && key.ends_with("u4:1234|mission-9"));
}

#[test]
fn t190_the_modal_timestamps_are_readable() {
    assert_eq!(ago(1_000_000.0, 1_000_000.0), "just now");
    assert_eq!(ago(1_000_000.0, 970_000.0), "30s ago");
    assert_eq!(ago(1_000_000.0, 700_000.0), "5m ago");
    assert_eq!(ago(10_000_000.0, 2_800_000.0), "2h ago");
    // A clock that moved backwards must not print a negative age.
    assert_eq!(ago(1_000.0, 9_000.0), "just now");
    assert_eq!(short_utc("2026-09-07T11:22:33Z"), "2026-09-07 11:22 UTC");
    assert_eq!(short_utc("not a date"), "not a date");
}

#[test]
fn t190_the_banner_names_the_cause_and_promises_only_what_is_true() {
    let one = banner_copy(1);
    assert!(
        one.contains("Another tab") && one.contains("merged"),
        "F-32: the banner must name the SECOND TAB as the cause, not server drift. {one}"
    );
    assert!(
        banner_copy(3).contains("3 other tabs"),
        "{}",
        banner_copy(3)
    );
}

/* ── the wasm write path, pinned ── */

/// RED before T-190: `run_save` read nothing before `save_state_as` put the whole key.
#[test]
fn t190_run_save_merges_the_stored_record_before_it_writes() {
    let live = persist_live();
    let run = only_item(&live, "async fn run_save(").to_string();
    let at_write = run
        .find("save_state_as")
        .unwrap_or_else(|| panic!("run_save must still write. run={run}"));
    let at_merge = run.find("merge_before_write").unwrap_or_else(|| {
        panic!("run_save must READ the record it is about to overwrite. run={run}")
    });
    assert!(
        at_merge < at_write,
        "the merge must run BEFORE the put, not after it. run={run}"
    );
    let before = only_item(&live, "async fn merge_before_write(").to_string();
    assert!(
        before.contains("read_raw") && before.contains("merge_stored"),
        "merge_before_write must read the stored record and merge it. before={before}"
    );
    assert!(
        before.contains("get_bytes"),
        "after a merge the document is the UNION, so the bytes must be re-encoded — otherwise \
             the write lands the pre-merge blob and the read was decoration. before={before}"
    );
    let merge = only_item(&live, "fn merge_stored(").to_string();
    assert!(
        merge.contains("apply_update"),
        "the merge must be MissionDocCore::apply_update — a CRDT union, never a JSON diff. \
             merge={merge}"
    );
}

/// RED before T-190: nothing on the write path knew another tab existed.
#[test]
fn t190_a_second_tab_cannot_silently_overwrite_the_first() {
    let live = persist_live();
    let run = only_item(&live, "async fn run_save(").to_string();
    assert!(
        run.contains("tab_lock") || run.contains("may_write"),
        "run_save must consult the cross-tab role before writing. run={run}"
    );
    assert!(
        run.contains("Defer") && run.contains("install_pending"),
        "a deferred save must be RE-ARMED, not dropped: the read-only tab inherits the writer \
             role when the other tab closes and its work has to survive to that point. run={run}"
    );
}

/// The channel and the lock both go through `Reflect`, per `core::client.rs`. A `web_sys::`
/// path for either is a `Cargo.toml` / RUSTFLAGS change this slice does not own.
#[test]
fn t190_the_channel_and_the_lock_follow_the_client_rs_precedent() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/tab_lock/live.rs"
    )));
    let prod = src.split("#[cfg(test)]").next().expect("test module");
    assert!(
        prod.contains("BroadcastChannel") && prod.contains("Reflect::get"),
        "the channel must be reached through js_sys::Reflect"
    );
    assert!(
        prod.contains("\"locks\"") && prod.contains("navigator"),
        "the writer role must be a navigator.locks lock, so a crashed tab releases it"
    );
    assert!(
        !prod.contains("web_sys::BroadcastChannel") && !prod.contains("web_sys::LockManager"),
        "neither binding is in this crate's web-sys feature list; adding one is a Cargo.toml \
             change T-190 does not own"
    );
    // The editor is a client-side route, so `join` must be able to let go of a PREVIOUS
    // mission — otherwise one page that visits mission A and then B holds A's writer role for
    // its whole life and B gets no presence at all.
    // `pub fn join(mission_id:` — the native stub is `join(_mission_id:`, and `only_item`
    // refuses an ambiguous marker rather than picking one of two.
    let src = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/tab_lock/live.rs"
    )));
    let join = only_item(&src, "pub fn join(mission_id:").to_string();
    assert!(
        join.contains("release"),
        "join must release the previous mission's lock and channel. join={join}"
    );
}

/// RED before T-190: `ConflictInfo` was `payload_json` + `semver` and nothing else.
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

/// RED before T-190: "Load server version" was the affirmative `bg-primary` button and no copy
/// anywhere said it destroys the local document.
#[test]
fn t190_load_server_version_is_marked_destructive() {
    let dialog = only_item(&overlays_src(), "pub(crate) fn ConflictDialog(").to_string();
    // `rfind`, so the needle is the button's visible LABEL and not its `aria-label` — the two
    // now differ, and slicing at the attribute would cut the arm before its own `class`.
    let at = dialog
        .rfind("Load server version")
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
    for needle in [
        "local_objects",
        "server_objects",
        "local_saved",
        "server_saved",
    ] {
        assert!(
            dialog.contains(needle),
            "the dialog must RENDER {needle}, not merely receive it. {dialog}"
        );
    }
}

/// T-946.51 — RED before T-190: `SAVE_IN_FLIGHT` was `.set(true)` in `run_save`, `.set(false)`
/// in `SaveFlightGuard::drop`, and read by nothing, while two comments credited it with the
/// serialisation `lock_for(id)` performs.
#[test]
fn t946_51_save_in_flight_is_read_by_something() {
    let live = persist_live();
    let n = live
        .match_indices("SAVE_IN_FLIGHT")
        .filter(|(i, _)| {
            let tail = &live[*i..];
            !tail.starts_with("SAVE_IN_FLIGHT.set(")
                && !tail.starts_with("SAVE_IN_FLIGHT: Cell<bool>")
                && !tail.starts_with("SAVE_IN_FLIGHT: Cell < bool >")
        })
        .count();
    assert!(
        n > 0,
        "SAVE_IN_FLIGHT is written and never read — either make it load-bearing or delete it \
             (T-946.51)"
    );
    let flight = only_item(&live, "pub fn save_in_flight(").to_string();
    assert!(
        flight.contains("SAVE_IN_FLIGHT"),
        "the reader must be the flag itself, not a second source. flight={flight}"
    );
    let src = persist_src();
    assert!(
        !src.contains("serialized by SAVE_IN_FLIGHT"),
        "the pagehide comment must not credit SAVE_IN_FLIGHT with serialisation that \
             `lock_for(id)` performs (T-946.51)"
    );
    // And the reader has to be the one that needs it: a peer's `saved` pull must not mutate the
    // live document between this tab's own encode and its put.
    let pull = only_item(&live, "pub fn pull_peer_record(").to_string();
    assert!(
        pull.contains("save_in_flight"),
        "the peer pull must stand down while this tab's own write is in flight. pull={pull}"
    );
}

/// T-946.54 — RED before T-190: the `UNREADABLE` insert came after 80 + 160 + 320 ms of
/// backoff, so `run_save` passed the T-374 guard for the whole window.
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
    assert!(
        item.contains("remove"),
        "a read that succeeds on retry must clear the latch, or one transient blip disables \
             autosave for the page lifetime. item={item}"
    );
}
