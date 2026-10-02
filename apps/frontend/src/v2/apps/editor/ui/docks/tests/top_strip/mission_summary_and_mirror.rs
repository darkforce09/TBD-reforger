//! Mission summary and mirror tests for the top command strip.

use super::{
    census_from_rows, draft_recency_phrase, format_draft_recency, hhmm_to_minutes,
    is_mission_row_id, minutes_to_hhmm, mirror_failure_message, normalize_clock, summary_line,
    MirrorState, SlotCensus, CENSUS_SIDES, MIRROR_DEBOUNCE_MS, MIRROR_TIME, MIRROR_WEATHER,
};
use crate::v2::apps::editor::ui::outliner::outliner::{FactionRow, SquadRow};

// ── T-659 census/summary fixtures ────────────────────────────────────────────────────────────

/// One faction row carrying a side `key` (`BLUFOR`/`OPFOR`/`INDFOR`). Ids mirror the live shape
/// `editor_ops::ensure_side_faction` mints (`faction-{SIDE}`), but the census keys off `key`, not
/// the id, so any id works — the test proves that by using the real shape.
fn faction(id: &str, key: &str) -> FactionRow {
    FactionRow {
        id: id.to_string(),
        key: key.to_string(),
        name: key.to_string(),
        squad_ids: Vec::new(),
    }
}

fn squad(id: &str, faction_id: &str) -> SquadRow {
    SquadRow {
        id: id.to_string(),
        name: id.to_string(),
        faction_id: faction_id.to_string(),
        slot_ids: Vec::new(),
        leader_slot_id: String::new(),
        vehicle_ids: Vec::new(),
    }
}

/// Build `slot_squad_ids` — `n` slots pointing at `squad_id` — as `census_input` hands it over.
fn slots_in(squad_id: &str, n: usize) -> Vec<String> {
    vec![squad_id.to_string(); n]
}

#[test]
fn time_scrubber_roundtrip() {
    assert_eq!(minutes_to_hhmm(0), "00:00");
    assert_eq!(minutes_to_hhmm(360), "06:00");
    assert_eq!(minutes_to_hhmm(1439), "23:59");
    assert_eq!(hhmm_to_minutes("06:00"), Some(360));
    assert_eq!(hhmm_to_minutes("23:59"), Some(1439));
    assert_eq!(hhmm_to_minutes("24:00"), None);
    assert_eq!(hhmm_to_minutes("nope"), None);
    for m in [0u32, 1, 59, 60, 719, 1439] {
        assert_eq!(hhmm_to_minutes(&minutes_to_hhmm(m)), Some(m));
    }
}

/// T-192 — `missions.time_of_day` is a Postgres `time`, so the row hydrate puts `HH:MM:SS` in
/// the document. The scrubber has to read it, or a reload silently shows 06:00 for a mission
/// set to 21:45 — the same reverted-setting symptom the ticket removes.
#[test]
fn scrubber_reads_the_row_hydrate_clock() {
    assert_eq!(hhmm_to_minutes("21:45:00"), Some(1305));
    assert_eq!(hhmm_to_minutes("06:00:00"), Some(360));
    assert_eq!(hhmm_to_minutes("00:00:59"), Some(0));
    // Still a clock parser, not a "contains digits" parser.
    assert_eq!(hhmm_to_minutes("12:00:60"), None);
    assert_eq!(hhmm_to_minutes("12:00:00:00"), None);
    assert_eq!(hhmm_to_minutes("12"), None);
    assert_eq!(hhmm_to_minutes("12:0a"), None);
    assert_eq!(hhmm_to_minutes(""), None);
}

/// What [`super::RowMirror::set_time`] sends to `PATCH /missions/{id}`: a canonical `HH:MM`, or
/// nothing at all. A half-typed clock must never reach the row.
#[test]
fn normalize_clock_is_the_patch_shape() {
    assert_eq!(normalize_clock("21:45").as_deref(), Some("21:45"));
    assert_eq!(normalize_clock("21:45:00").as_deref(), Some("21:45"));
    assert_eq!(normalize_clock("6:5").as_deref(), Some("06:05"));
    for bad in ["", "2", "24:00", "12:60", "noon", "12:00:00:00"] {
        assert_eq!(normalize_clock(bad), None, "{bad:?} must not be PATCHed");
    }
}

/// T-804 (F-24) — the draft-safety chip's recency ladder, the property the scripted acceptance
/// asserts against: "recency <=6s after an edit" and "counts up over 30s idle" are statements
/// about THIS pure function, so they are proven here without a browser.
#[test]
fn draft_recency_reads_up_the_ladder() {
    // The sub-5s / negative band degrades to "just now" — the acceptance's "chip shows 'draft
    // saved'" at <=6s recency is this arm plus the "6s ago" one below (both begin "Draft saved").
    assert_eq!(draft_recency_phrase(0.0), "just now");
    assert_eq!(draft_recency_phrase(4_999.0), "just now");
    // Backwards clock skew must not print "-3s ago"; it floors to "just now".
    assert_eq!(draft_recency_phrase(-3_000.0), "just now");
    // Seconds — the band the acceptance's <=6s recency lands in once past the just-now floor.
    assert_eq!(draft_recency_phrase(5_000.0), "5s ago");
    assert_eq!(draft_recency_phrase(6_000.0), "6s ago");
    assert_eq!(draft_recency_phrase(59_000.0), "59s ago");
    // It COUNTS UP: 30 s idle after a flush reads a larger number than 6 s did — monotone.
    assert_eq!(draft_recency_phrase(30_000.0), "30s ago");
    // Minutes, then hours — whole units only (1 Hz tick, no finer claim).
    assert_eq!(draft_recency_phrase(60_000.0), "1m ago");
    assert_eq!(draft_recency_phrase(3_599_000.0), "59m ago");
    assert_eq!(draft_recency_phrase(3_600_000.0), "1h ago");
    // The full chip text carries the F-24 pre-approved "Draft saved " prefix.
    assert_eq!(format_draft_recency(6_000.0), "Draft saved 6s ago");
    assert_eq!(format_draft_recency(0.0), "Draft saved just now");
}

/// T-746 — the row-id predicate is crate-visible so `eden_settings` does not keep a twin.
#[test]
fn t746_row_id_predicate_is_crate_visible() {
    use crate::v2::core::test_support::class_r_scrub::live_code;
    let src = live_code(super::test_source::top_strip_source());
    assert!(
        src.contains("pub(crate) fn is_mission_row_id"),
        "T-746: is_mission_row_id must be pub(crate), not a private twin in eden_settings"
    );
}

/// The mirror only fires on a real row. `mission_editor` falls back to `draft` and the editor
/// gate mounts on a smoke id — a PATCH there is a guaranteed 400 and pure console noise.
#[test]
fn only_a_uuid_route_id_gets_mirrored() {
    assert!(is_mission_row_id("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
    assert!(is_mission_row_id("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
    for bad in [
        "",
        "draft",
        "smoke",
        "3f2504e0-4f89-11d3-9a0c-0305e82c330",   // short
        "3f2504e0-4f89-11d3-9a0c-0305e82c33011", // long
        "3f2504e0x4f89-11d3-9a0c-0305e82c3301",  // dash in the wrong place
        "3f2504e0-4f89-11d3-9a0c-0305e82c330g",  // non-hex
    ] {
        assert!(!is_mission_row_id(bad), "{bad:?} is not a mission row id");
    }
}

/// The columns the mirror PATCHes, and the words the failure toast uses for them. Pinned
/// because the column half is the API contract (`PatchMissionInput`) and the label half is what
/// the author reads — `viewDistance` / `thermals` are absent because T-193 stopped the editor
/// authoring them at all, not because the mirror declined to carry them.
#[test]
fn mirrored_fields_are_the_two_row_columns() {
    assert_eq!(MIRROR_TIME.column, "time_of_day");
    assert_eq!(MIRROR_WEATHER.column, "weather");
    assert_eq!(MIRROR_TIME.label, "Time of day");
    assert_eq!(MIRROR_WEATHER.label, "Weather");
    assert_ne!(MIRROR_TIME.column, MIRROR_WEATHER.column, "one queue each");
}

/// A failed mirror must SAY so — the shipped version only `warn!`ed, which is how a
/// mission_maker editing someone else's live mission watched the setting apply and revert with
/// no feedback at all. Every failure names the setting and says it will revert.
#[test]
fn every_mirror_failure_names_the_setting_and_the_revert() {
    for field in [MIRROR_TIME, MIRROR_WEATHER] {
        for err in [
            (403u16, Some("not your mission".to_string())),
            (400, Some("invalid weather".to_string())),
            (500, None),
            (0, None), // transport: no response at all
        ] {
            let msg = mirror_failure_message(field, &err);
            assert!(
                msg.to_lowercase().contains(&field.label.to_lowercase()),
                "{err:?} must name the setting: {msg}"
            );
            assert!(
                msg.contains("revert"),
                "{err:?} must warn of the revert: {msg}"
            );
        }
    }
}

/// 403 and "the server fell over" call for different action, so they must not read the same.
/// The ownership refusal is structural — `PATCH /missions/:id` gates on `can_edit` (author or
/// admin) while the editor route gates on role — so retrying cannot help and the text must not
/// suggest it. Everything else is worth another go and carries what the server said.
#[test]
fn forbidden_and_transport_failures_read_differently() {
    let denied = mirror_failure_message(MIRROR_TIME, &(403, Some("not your mission".into())));
    let dropped = mirror_failure_message(MIRROR_TIME, &(0, None));
    assert_ne!(denied, dropped);
    assert!(
        denied.contains("author"),
        "403 must name the cause: {denied}"
    );
    assert!(
        !denied.contains("try again"),
        "403 is not retryable: {denied}"
    );
    assert!(
        dropped.contains("try again"),
        "a transport failure is: {dropped}"
    );
    assert!(
        dropped.contains("the server did not respond"),
        "a bodyless failure still says what happened: {dropped}"
    );
    // A backend message is surfaced verbatim (capitalized), not flattened to one house string.
    let bad = mirror_failure_message(MIRROR_WEATHER, &(400, Some("invalid weather".into())));
    assert!(bad.contains("Invalid weather"), "{bad}");
}

/// The rate bound. A held scrubber emits ~30 distinct values a second; the window has to be long
/// enough to swallow that burst and short enough that a settled value lands while the author is
/// still looking at the dialog.
#[test]
fn mirror_debounce_bounds_the_patch_rate() {
    assert!(
        MIRROR_DEBOUNCE_MS >= 200,
        "a 30 Hz scrub must collapse to one PATCH"
    );
    assert!(MIRROR_DEBOUNCE_MS <= 1000, "a settle must feel immediate");
}

/// A held scrubber: 30 distinct values inside one debounce window must reach the wire as ONE
/// PATCH carrying the value the author stopped on. Every intermediate value is in the document
/// already, so none of them is worth a round trip — and 30 of them racing is how the row ends up
/// holding one the author scrubbed past.
#[test]
fn a_burst_collapses_to_the_settled_value() {
    let mut f = MirrorState::default();
    let mut rearms = 0;
    for m in 0..30u32 {
        if f.queue(minutes_to_hhmm(360 + m)) {
            rearms += 1; // each commit restarts the window; the timer fires once, at the end
        }
    }
    assert_eq!(rearms, 30, "every distinct value re-arms");
    let (generation, value) = f
        .take_for_send()
        .expect("the window closed with work queued");
    assert_eq!(
        value, "06:29",
        "the wire gets the settled value, not the first"
    );
    assert_eq!(f.take_for_send(), None, "and only that one");
    assert!(
        !f.settle(generation, value, true).queued,
        "nothing left over"
    );
    assert_eq!(f.last, "06:29");
}

/// The single-flight rule, which is what actually makes out-of-order landing unreachable: while
/// one PATCH is on the wire, a newer value cannot start a second one. It waits, and the
/// completion hands it the slot — so the row's last write is always the author's last edit.
#[test]
fn a_second_patch_cannot_start_while_one_is_in_flight() {
    let mut f = MirrorState::default();
    assert!(f.queue("06:00".into()));
    let (first, first_value) = f.take_for_send().expect("first goes out");

    assert!(f.queue("21:45".into()), "the author moves on mid-flight");
    assert_eq!(
        f.take_for_send(),
        None,
        "the window may close again, but the slot is busy"
    );

    // The first response comes back. It no longer speaks for the field, so it must not write
    // `last` — otherwise the losing value is what the next hydrate believes.
    let settled = f.settle(first, first_value, true);
    assert!(settled.stale, "the author has moved past 06:00");
    assert!(settled.queued, "21:45 is still waiting");
    assert_eq!(f.last, "", "a stale response must not record a value");

    let (second, second_value) = f.take_for_send().expect("the slot is free now");
    assert_eq!(second_value, "21:45");
    assert!(second > first, "generations are monotonic");
    assert!(!f.settle(second, second_value, true).stale);
    assert_eq!(f.last, "21:45", "the row ends on the author's last edit");
}

/// A stale FAILURE is just as silent as a stale success: it must not clear `last` (which
/// describes a different generation) and must not toast (its successor will). The newest
/// failure always speaks — that is the whole MAJOR.
#[test]
fn only_the_newest_generation_reports_a_failure() {
    let mut f = MirrorState::default();
    assert!(f.queue("clear".into()));
    let (first, first_value) = f.take_for_send().unwrap();
    assert!(!f.settle(first, first_value, true).stale);
    assert_eq!(f.last, "clear");

    assert!(f.queue("overcast".into()));
    let (second, second_value) = f.take_for_send().unwrap();
    assert!(f.queue("dense_fog".into()), "author moves on mid-flight");
    let stale_failure = f.settle(second, second_value, false);
    assert!(stale_failure.stale, "no toast for a value already replaced");
    assert_eq!(f.last, "clear", "a stale failure must not rewrite last");

    let (third, third_value) = f.take_for_send().unwrap();
    let live_failure = f.settle(third, third_value, false);
    assert!(!live_failure.stale, "THIS one must reach the user");
    assert!(!live_failure.queued);
    // Cleared so the very next commit of "dense_fog" retries rather than deduping away.
    assert_eq!(f.last, "");
    assert!(f.queue("dense_fog".into()), "the retry is not deduped away");
}

/// The T-192 dedupe, extended to cover the queue: a rebuild replaying a value, or a scrub that
/// wanders back to what is already queued, must not cost a PATCH or bump the generation.
#[test]
fn an_unchanged_value_costs_nothing() {
    let mut f = MirrorState::default();
    assert!(f.queue("06:00".into()));
    assert!(!f.queue("06:00".into()), "same value, already queued");
    assert_eq!(f.generation, 1, "a no-op must not bump the generation");

    let (g, v) = f.take_for_send().unwrap();
    f.settle(g, v, true);
    assert!(!f.queue("06:00".into()), "same value, already on the row");
    assert_eq!(f.take_for_send(), None, "and nothing to send");
}

// ── T-659 — census derivation ────────────────────────────────────────────────────────────────

/// The header example, verbatim: `WEST 78 · EAST 74 · IND 8 · TOTAL 160`. A multi-side roster
/// tallies each side off its faction `key`, and the total equals the slot set. This is the pure
/// derivation the badge renders — no doc, no wasm.
#[test]
fn census_counts_each_side_and_totals() {
    let factions = [
        faction("faction-BLUFOR", "BLUFOR"),
        faction("faction-OPFOR", "OPFOR"),
        faction("faction-INDFOR", "INDFOR"),
    ];
    let squads = [
        squad("sq-w", "faction-BLUFOR"),
        squad("sq-e", "faction-OPFOR"),
        squad("sq-i", "faction-INDFOR"),
    ];
    let mut slot_squad_ids = slots_in("sq-w", 78);
    slot_squad_ids.extend(slots_in("sq-e", 74));
    slot_squad_ids.extend(slots_in("sq-i", 8));

    let c = census_from_rows(&factions, &squads, &slot_squad_ids);
    assert_eq!(
        c,
        SlotCensus {
            west: 78,
            east: 74,
            ind: 8,
            unassigned: 0,
            total: 160,
        }
    );
    // The invariant that makes "counts don't add up" unrepresentable.
    assert_eq!(c.west + c.east + c.ind + c.unassigned, c.total);
}

/// The zero state — an empty document (no factions, no squads, no slots) — is a clean all-zero
/// census, not a panic or an unassigned pile. This is the mount-time state the badge renders
/// before the first place.
#[test]
fn census_zero_state_is_all_zero() {
    let c = census_from_rows(&[], &[], &[]);
    assert_eq!(c, SlotCensus::default());
    assert_eq!(c.total, 0);
    assert_eq!(c.unassigned, 0);
}

/// Unassigned handling — the whole point of deriving the census off the snapshot rather than a
/// rule. Three ways a slot resolves to no side, all landing in `unassigned` and none in a side
/// bucket: (1) a `squadId` with no squad in the map (the seed-slot "dangling squadId" case,
/// `editor_ops.rs:15`); (2) a squad under a faction with an empty side `key`; (3) an empty
/// `squadId`. The malformed state is a bucket, not a caught error.
#[test]
fn census_unassigned_covers_every_unresolved_slot() {
    let factions = [
        faction("faction-BLUFOR", "BLUFOR"),
        // A faction the doc kept but whose side key never got written.
        faction("faction-mystery", ""),
    ];
    let squads = [
        squad("sq-w", "faction-BLUFOR"),
        squad("sq-mystery", "faction-mystery"),
    ];
    let mut ids = slots_in("sq-w", 5); // → WEST
    ids.extend(slots_in("sq-ghost", 3)); // dangling squadId (no such squad)
    ids.extend(slots_in("sq-mystery", 2)); // squad under a keyless faction
    ids.push(String::new()); // slot with no squadId at all

    let c = census_from_rows(&factions, &squads, &ids);
    assert_eq!(c.west, 5);
    assert_eq!(c.east, 0);
    assert_eq!(c.ind, 0);
    assert_eq!(c.unassigned, 6, "3 ghost + 2 keyless + 1 empty");
    assert_eq!(c.total, 11);
    assert_eq!(c.west + c.east + c.ind + c.unassigned, c.total);
}

/// A single-side roster is exactly what it says: WEST populated, EAST/IND zero, no unassigned.
/// Guards against an off-by-one that would leak the other sides' zeros into `unassigned`.
#[test]
fn census_single_side_leaves_others_zero() {
    let factions = [faction("faction-OPFOR", "OPFOR")];
    let squads = [squad("sq-e", "faction-OPFOR")];
    let c = census_from_rows(&factions, &squads, &slots_in("sq-e", 12));
    assert_eq!(c.east, 12);
    assert_eq!(c.west, 0);
    assert_eq!(c.ind, 0);
    assert_eq!(c.unassigned, 0);
    assert_eq!(c.total, 12);
}

/// The side→label table is the vocabulary the community naming convention rides. Pin it so a
/// rename (WEST→BLUEFOR, say) is a deliberate, test-breaking act, not a silent drift — the WOG
/// lesson the ticket calls out. `count_for_key` must agree with the table.
#[test]
fn census_side_labels_are_pinned() {
    assert_eq!(
        CENSUS_SIDES,
        [("BLUFOR", "WEST"), ("OPFOR", "EAST"), ("INDFOR", "IND")]
    );
    let c = SlotCensus {
        west: 1,
        east: 2,
        ind: 3,
        unassigned: 0,
        total: 6,
    };
    assert_eq!(c.count_for_key("BLUFOR"), 1);
    assert_eq!(c.count_for_key("OPFOR"), 2);
    assert_eq!(c.count_for_key("INDFOR"), 3);
    assert_eq!(
        c.count_for_key("CIV"),
        0,
        "a non-side key counts to no bucket"
    );
}

// ── T-659 — summary-line format (STABLE — other tools parse this) ─────────────────────────────

/// **GOLDEN — the community naming format. Changing this string is a breaking change.**
///
/// The full-roster line, matching the ticket's illustrative shape (`"COOP 160 on Everon — WEST
/// 78 v EAST 74 (+8 IND)"`). This golden pins the em-dash, the ` v ` core, the `(+ IND)` suffix,
/// the mode prefix, and the `everon`→`Everon` label — the load-bearing punctuation
/// `summary_line` documents as the parser contract.
#[test]
fn summary_golden_full_roster_with_mode() {
    let c = SlotCensus {
        west: 78,
        east: 74,
        ind: 8,
        unassigned: 0,
        total: 160,
    };
    assert_eq!(
        summary_line(&c, "everon", Some("COOP")),
        "COOP 160 on Everon — WEST 78 v EAST 74 (+8 IND)"
    );
}

/// No mode present → no mode segment, and the ` v ` core plus the two anchor words `on` / ` v `
/// are still there so a parser can locate the fields regardless of the mode's absence. This is
/// the common case (game mode is not a first-class editor field today).
#[test]
fn summary_omits_mode_when_absent() {
    let c = SlotCensus {
        west: 10,
        east: 10,
        ind: 0,
        unassigned: 0,
        total: 20,
    };
    let line = summary_line(&c, "arland", None);
    assert_eq!(line, "20 on Arland — WEST 10 v EAST 10");
    assert!(!line.contains(" (+"), "no IND suffix when IND is zero");
    assert!(line.contains(" on "), "the `on` anchor is always present");
    assert!(line.contains(" v "), "the ` v ` anchor is always present");
}

/// The optional suffixes are strictly APPEND-ONLY and ordered `(+i IND)` then `(u unassigned)`,
/// so adding either never moves an earlier field. Both present at once here; the IND suffix
/// precedes the unassigned one.
#[test]
fn summary_suffixes_are_append_only_and_ordered() {
    let c = SlotCensus {
        west: 5,
        east: 4,
        ind: 3,
        unassigned: 2,
        total: 14,
    };
    let line = summary_line(&c, "everon", Some("TVT"));
    assert_eq!(
        line,
        "TVT 14 on Everon — WEST 5 v EAST 4 (+3 IND) (2 unassigned)"
    );
    // The core prefix is byte-identical to the no-suffix line — proof the suffixes only append.
    let core = "TVT 14 on Everon — WEST 5 v EAST 4";
    assert!(line.starts_with(core), "suffixes must not perturb the core");
    let ind_at = line.find("(+3 IND)").expect("IND suffix present");
    let una_at = line
        .find("(2 unassigned)")
        .expect("unassigned suffix present");
    assert!(ind_at < una_at, "IND suffix precedes the unassigned suffix");
}

/// A blank/unknown terrain still yields a parseable line (`Unknown` placeholder), never an empty
/// map name that would leave the `on ` anchor dangling.
#[test]
fn summary_handles_blank_terrain() {
    let c = SlotCensus::default();
    assert_eq!(summary_line(&c, "", None), "0 on Unknown — WEST 0 v EAST 0");
}

/// The census and the summary compose end-to-end: the roster the pure census produces is exactly
/// what the summary reports. This is the live-update pin's pure half — the header memos feed
/// `census_from_rows`'s output straight into `summary_line`, and the reactivity source is the
/// `doc_tick` channel documented on the `census`/`summary` memos (`refresh_docks` →
/// `editor_ops.rs:2660`), which native tests cannot drive but which the memo wiring pins.
#[test]
fn census_and_summary_compose() {
    let factions = [
        faction("faction-BLUFOR", "BLUFOR"),
        faction("faction-INDFOR", "INDFOR"),
    ];
    let squads = [
        squad("sq-w", "faction-BLUFOR"),
        squad("sq-i", "faction-INDFOR"),
    ];
    let mut ids = slots_in("sq-w", 2);
    ids.extend(slots_in("sq-i", 1));
    let c = census_from_rows(&factions, &squads, &ids);
    assert_eq!(
        summary_line(&c, "everon", None),
        "3 on Everon — WEST 2 v EAST 0 (+1 IND)"
    );
}

/// FIRE THE RULE ONCE (perturb / fail / restore). This census REPLACES the two MissionAnalyzer
/// rules by making the malformed state a bucket rather than a caught error, so "firing the rule"
/// is: a clean roster shows `unassigned == 0`; perturbing a slot onto a dangling squad makes the
/// census REPORT the orphan (`unassigned > 0`, and the badge would light UNA); restoring the slot
/// to a real squad returns the census to clean. The malformed state is observable by
/// construction — there is no analyzer pass that could fail to run.
#[test]
fn census_fires_on_an_orphan_then_clears_on_restore() {
    let factions = [faction("faction-BLUFOR", "BLUFOR")];
    let squads = [squad("sq-w", "faction-BLUFOR")];

    // Clean: every slot resolves to WEST.
    let clean = census_from_rows(&factions, &squads, &slots_in("sq-w", 4));
    assert_eq!(clean.unassigned, 0, "clean roster: nothing unassigned");
    assert_eq!(clean.west, 4);

    // Perturb: one slot now points at a squad that isn't in the map (the orphan the old rule
    // existed to catch). The census FIRES — the orphan surfaces in `unassigned`.
    let mut perturbed_ids = slots_in("sq-w", 3);
    perturbed_ids.push("sq-deleted".to_string());
    let perturbed = census_from_rows(&factions, &squads, &perturbed_ids);
    assert_eq!(
        perturbed.unassigned, 1,
        "the orphan is reported, not silently dropped"
    );
    assert_eq!(perturbed.west, 3);
    assert_eq!(perturbed.total, 4, "total still counts every slot");

    // Restore: refile the orphan back onto the real squad — the census returns to clean.
    let restored = census_from_rows(&factions, &squads, &slots_in("sq-w", 4));
    assert_eq!(restored, clean, "restoring clears the fired state");
}
