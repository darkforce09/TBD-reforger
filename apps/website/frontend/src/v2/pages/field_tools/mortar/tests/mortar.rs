//! The guards on the mortar calculator: the save round trip, the grid encoding, the preview
//! projection and the load-time hydration latch.

use super::*;

/// The exact body `GET /events/{id}/fire-missions` returned on the live dev API on
/// 2026-07-31, for the fire mission posted by the round-trip probe:
/// FP (1000, 2000) → TGT (2200, 1800), `M252 81mm`. Captured verbatim — the field names, the
/// envelope shape and the absence of `time_of_flight_s` are all server truth, not a guess.
const LIVE_LIST: &str = r#"{"data":[{"id":"97176662-5589-4831-85e5-61f2a7bd8597","event_id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","created_by":"000000000000000001","weapon_system":"M252 81mm","fp_grid":"1000, 2000","target_grid":"2200, 1800","distance_m":1217,"azimuth_deg":99.5,"elevation_mils":1315,"created_at":"2026-07-31T02:16:52.695935Z"}]}"#;

/// **The round trip, over real server bytes.** Build the body the page posts, hand the grids
/// it produced to the response the server actually gave back, and require the four inputs and
/// the three persisted numbers to come out the far side unchanged.
///
/// This is not an assertion a page that merely renders can satisfy: perturb `fmt_grid` (drop
/// the separator, quantise to a six-figure grid, swap the
/// axes) and the restored coordinates stop matching the posted ones while every view in this
/// module still builds and still shows a solution.
#[test]
fn a_posted_solution_comes_back_from_the_server_with_the_same_numbers() {
    let fp = (1000.0, 2000.0);
    let tgt = (2200.0, 1800.0);
    let body = save_body(
        "M252 81mm",
        fp,
        tgt,
        Some("c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7"),
    );

    // What went out.
    assert_eq!(body["fp_grid"], "1000, 2000");
    assert_eq!(body["target_grid"], "2200, 1800");

    // What the server stored and handed back.
    let list: DataEnvelope<SavedFire> = serde_json::from_str(LIVE_LIST).expect("live list decodes");
    let row = list.data.last().expect("one saved fire mission");
    assert_eq!(row.fp_grid, body["fp_grid"].as_str().unwrap());
    assert_eq!(row.target_grid, body["target_grid"].as_str().unwrap());

    // What the page shows after a reload.
    let r = restore(row).expect("a row this module wrote restores");
    assert_eq!(r.fp, fp, "FP did not survive the round trip");
    assert_eq!(r.tgt, tgt, "TGT did not survive the round trip");
    assert_eq!(r.shown.weapon_system, "M252 81mm");
    assert_eq!(r.shown.distance_m, 1217);
    assert_eq!(r.shown.azimuth_deg, 99.5);
    assert_eq!(r.shown.elevation_mils, 1315);
    assert_eq!(
        r.shown.saved_at.as_deref(),
        Some("2026-07-31T02:16:52.695935Z")
    );
}

/// **The pre-existing row.** [`LIVE_LIST`] was captured before the later columns
/// existed, so it is the exact wire shape of every fire mission already in the table: no
/// `fp_x`, no `charge`, no `time_of_flight_s`. It must still decode, still restore its
/// coordinates (through the [`fmt_grid`] fallback, the only record it has of them), and still
/// render `—` for the two numbers it does not carry.
///
/// This is the regression that the obvious version of this ticket breaks: read the new columns,
/// delete the grid fallback as "superseded", and every historical row silently stops restoring.
/// Nothing throws — the list still renders, the rows are still there, and clicking one just
/// does nothing.
#[test]
fn a_row_saved_before_the_migration_still_restores_and_shows_no_tof_or_charge() {
    let list: DataEnvelope<SavedFire> = serde_json::from_str(LIVE_LIST).unwrap();
    let row = &list.data[0];
    // The columns are absent from the capture, not null-and-present.
    assert_eq!(
        (row.fp_x, row.fp_y, row.tgt_x, row.tgt_y),
        (None, None, None, None)
    );
    assert_eq!((row.charge, row.time_of_flight_s), (None, None));

    let r = restore(row)
        .expect("a row saved before the columns existed still restores via the grid encoding");
    assert_eq!(r.fp, (1000.0, 2000.0), "coordinates come from fp_grid");
    assert_eq!(r.tgt, (2200.0, 1800.0), "coordinates come from target_grid");
    assert_eq!(r.shown.time_of_flight_s, None);
    assert_eq!(r.shown.charge, None);
    // …while a live solve does carry both, so the two sources stay distinguishable.
    let solved: FireSolution = serde_json::from_str(
        r#"{"weapon_system":"M252 81mm","distance_m":1217,"azimuth_deg":99.5,"azimuth_mils":1768,"elevation_mils":1315,"charge":2,"time_of_flight_s":29.4}"#,
    )
    .unwrap();
    assert_eq!(Shown::from(&solved).time_of_flight_s, Some(29.4));
    assert_eq!(Shown::from(&solved).charge, Some(2));
}

/// **The row a save produces once the columns exist.** Captured from
/// `GET /events/{id}/fire-missions` against the live handler in
/// `apps/website/api_v2/tests/fire_mission_solution.rs`, which asserts these same values against the
/// database row itself.
///
/// The card a reload builds from this must be the card the live solve built: same charge, same
/// TOF, no `—` anywhere. If any of the seven columns stops being written, or stops being
/// projected by the SELECT, this goes red on the field that went missing rather than on a
/// vague "restore returned None".
const LIVE_LIST_T587: &str = r#"{"data":[{"id":"5c2e8b4a-1f77-4a10-9d3e-2b6c7f0a1e44","event_id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","created_by":"000000000000000001","weapon_system":"M252 81mm","fp_grid":"1000, 2000","target_grid":"2200, 1800","distance_m":1217,"azimuth_deg":99.5,"elevation_mils":1315,"fp_x":1000.0,"fp_y":2000.0,"tgt_x":2200.0,"tgt_y":1800.0,"azimuth_mils":1768,"charge":2,"time_of_flight_s":29.4,"created_at":"2026-08-01T10:04:11.512004Z"}]}"#;

#[test]
fn a_row_saved_after_the_migration_restores_the_whole_solution() {
    let list: DataEnvelope<SavedFire> = serde_json::from_str(LIVE_LIST_T587).unwrap();
    let row = &list.data[0];
    let r = restore(row).expect("a fully-populated row restores");

    // Coordinates come from the COLUMNS now. Proven by breaking the encoding: a row whose
    // grid strings no longer parse still restores, which was impossible before this ticket.
    let mut no_grid = row.clone();
    no_grid.fp_grid = "GRID 012345".into();
    no_grid.target_grid = "GRID 012845".into();
    let r2 = restore(&no_grid).expect("the numeric columns carry it without the encoding");
    assert_eq!((r2.fp, r2.tgt), (r.fp, r.tgt));
    assert_eq!(r.fp, (1000.0, 2000.0));
    assert_eq!(r.tgt, (2200.0, 1800.0));

    // The three numbers that had no column at first.
    assert_eq!(r.shown.charge, Some(2), "charge did not survive the reload");
    assert_eq!(
        r.shown.time_of_flight_s,
        Some(29.4),
        "TOF did not survive the reload"
    );
    assert_eq!(
        row.azimuth_mils,
        Some(1768),
        "the sight setting is on the row"
    );

    // …and the card is now indistinguishable from the freshly-computed one, which is the
    // whole ticket. Same source numbers, same card, minus the `saved_at` a live solve has
    // not earned yet.
    let solved: FireSolution = serde_json::from_str(
        r#"{"weapon_system":"M252 81mm","distance_m":1217,"azimuth_deg":99.5,"azimuth_mils":1768,"elevation_mils":1315,"charge":2,"time_of_flight_s":29.4}"#,
    )
    .unwrap();
    let mut fresh = Shown::from(&solved);
    assert_eq!(fresh.saved_at, None);
    fresh.saved_at = r.shown.saved_at.clone();
    assert_eq!(
        fresh, r.shown,
        "a restored solution must render as the same card as a fresh one"
    );
}

/// The grid encoding is the persistence of the operator's inputs, so it has to be lossless
/// over everything the number inputs can hold — negatives, fractions, zero and a coordinate
/// far outside any terrain.
#[test]
fn every_coordinate_the_inputs_accept_round_trips_through_the_grid_string() {
    for (x, y) in [
        (0.0, 0.0),
        (1000.0, 2000.0),
        (2200.5, 1800.25),
        (-750.0, 12800.0),
        (0.1, -0.1),
        (123456.789, 987654.321),
    ] {
        let s = fmt_grid(x, y);
        assert_eq!(
            parse_grid(&s),
            Some((x, y)),
            "({x}, {y}) did not survive as {s:?}"
        );
        assert_eq!(s.trim(), s, "the handler trims before storing: {s:?}");
    }
}

/// A grid this module did not write restores as "no coordinates", never as `(0, 0)`.
#[test]
fn a_foreign_grid_reference_restores_as_nothing_rather_than_as_the_origin() {
    for s in ["012345", "", "1000", "AB, CD", "1000, ", "NaN, 3"] {
        assert_eq!(parse_grid(s), None, "{s:?} should not parse");
    }
    let mut row: SavedFire = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
        .unwrap()
        .data
        .remove(0);
    row.fp_grid = "012345".into();
    assert_eq!(restore(&row), None);
}

/// `POST /fire-missions` requires all seven fields (`field_tools.rs:224-233`) and refuses a
/// blank `event_id` with a 400 (`:246-254`), so the no-operation body must omit the key
/// entirely rather than send `""`.
#[test]
fn the_post_body_carries_every_field_the_route_requires() {
    let b = save_body("M120 120mm", (1.0, 2.0), (3.0, 4.0), Some("  ev-1  "));
    for k in [
        "weapon_system",
        "fp_x",
        "fp_y",
        "tgt_x",
        "tgt_y",
        "fp_grid",
        "target_grid",
        "event_id",
    ] {
        assert!(b.get(k).is_some(), "missing {k} in {b}");
    }
    assert_eq!(b["weapon_system"], "M120 120mm");
    assert_eq!(b["fp_x"], 1.0);
    assert_eq!(b["tgt_y"], 4.0);
    assert_eq!(b["event_id"], "ev-1", "the id must be trimmed, not refused");

    for none in [None, Some(""), Some("   ")] {
        let b = save_body("M252 81mm", (1.0, 2.0), (3.0, 4.0), none);
        assert!(
            b.get("event_id").is_none(),
            "a blank event_id must be omitted, not sent: {b}"
        );
    }
}

/// **Claim 2's perturbation test.** The defect was a marker subtree that read no input, so the
/// assertion is that every input moves it: change one coordinate at a time and require the
/// projected position to change. Restore the fixed CSS (`top-1/4 left-1/3`) and this is the
/// test that goes red — a render assertion would not, because the markers rendered fine.
#[test]
fn both_preview_markers_move_when_any_input_moves() {
    let base = preview_pos((1000.0, 2000.0), (2200.0, 1800.0));
    for (fp, tgt, what) in [
        ((1500.0, 2000.0), (2200.0, 1800.0), "FP X"),
        ((1000.0, 2500.0), (2200.0, 1800.0), "FP Y"),
        ((1000.0, 2000.0), (2800.0, 1800.0), "TGT X"),
        ((1000.0, 2000.0), (2200.0, 1200.0), "TGT Y"),
    ] {
        assert_ne!(
            preview_pos(fp, tgt),
            base,
            "{what} did not move the preview"
        );
    }
    // Both markers stay inside the box at any separation, and north is up.
    for (fp, tgt) in [
        ((0.0, 0.0), (12800.0, 12800.0)),
        ((6400.0, 6400.0), (6401.0, 6400.5)),
        ((-5000.0, 9000.0), (5000.0, -9000.0)),
    ] {
        let (a, b) = preview_pos(fp, tgt);
        for (l, t) in [a, b] {
            assert!(
                (0.0..=100.0).contains(&l) && (0.0..=100.0).contains(&t),
                "{l},{t}"
            );
        }
        // Whichever point is further north gets the smaller `top`.
        if fp.1 > tgt.1 {
            assert!(a.1 < b.1, "north must be up");
        }
    }
    // A gun sitting on its own target has no scale to fit; both markers centre.
    assert_eq!(
        preview_pos((500.0, 500.0), (500.0, 500.0)),
        ((50.0, 50.0), (50.0, 50.0))
    );
}

/// **The measured race, pinned.** On 2026-07-31 a real browser against the live API showed a
/// cold session pick its operation and get "Enter coordinates and calculate to see solution."
/// back over a fire mission it already had in hand: `LocalResource` was still serving the
/// previous key's value, the once-per-operation latch fired against *that*, and the real rows
/// arriving a tick later were then correctly ignored as already-hydrated.
///
/// Delete the `batch.event_id != want` arm of [`hydration_step`] and the first case below goes
/// green-then-wrong exactly as it did in the browser: it latches, restores nothing, and no
/// render assertion anywhere can tell the difference between that and an operation with no
/// saved fire missions.
#[test]
fn hydration_refuses_a_batch_fetched_for_a_different_operation() {
    let row = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
        .unwrap()
        .data
        .remove(0);
    let want = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";

    // The value in flight belongs to "no operation" — the state a cold page starts in.
    let stale = SavedFor {
        event_id: None,
        rows: Vec::new(),
    };
    assert_eq!(
        hydration_step(&stale, want, &HashSet::new()),
        None,
        "a batch fetched for another operation must not hydrate AND must not latch"
    );

    // …and the batch that actually answers for `want` still hydrates afterwards, which is the
    // half that was broken: the latch had already been spent.
    let fresh = SavedFor {
        event_id: Some(want.to_string()),
        rows: vec![row],
    };
    let applied = hydration_step(&fresh, want, &HashSet::new())
        .expect("the batch for this operation must be acted on")
        .expect("its newest row must restore");
    assert_eq!(applied.fp, (1000.0, 2000.0));
    assert_eq!(applied.shown.distance_m, 1217);

    // Once done it is done — a refetch after a save must not clobber the fresh card.
    assert_eq!(
        hydration_step(&fresh, want, &HashSet::from([want.to_string()])),
        None
    );

    // An operation with nothing saved is a real answer: latch, restore nothing.
    let empty = SavedFor {
        event_id: Some(want.to_string()),
        rows: Vec::new(),
    };
    assert_eq!(hydration_step(&empty, want, &HashSet::new()), Some(None));
}

/// Switching operation away and back must NOT re-hydrate.
///
/// The latch used to be a single `Option<String>` slot holding the last operation hydrated, so
/// it only ever remembered one. Sequence A → B → A: hydrating B overwrote the memory of A, and
/// returning to A hydrated it a second time, dropping whatever the operator had typed in the
/// meantime over the saved solution. No data was lost from the server's point of view, which is
/// exactly why it went unnoticed.
///
/// Revert `already_hydrated` to a single slot and the final assertion here goes red.
#[test]
fn returning_to_an_operation_does_not_re_hydrate_over_unsaved_edits() {
    let row = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
        .unwrap()
        .data
        .remove(0);
    let a = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";
    let b = "00000000-0000-4000-7000-000000000001";
    let batch_a = SavedFor {
        event_id: Some(a.to_string()),
        rows: vec![row],
    };
    let batch_b = SavedFor {
        event_id: Some(b.to_string()),
        rows: Vec::new(),
    };

    // This is the real effect's state, threaded by hand.
    let mut seen: HashSet<String> = HashSet::new();

    // 1. Land on A: it hydrates, and A is latched.
    assert!(
        hydration_step(&batch_a, a, &seen).is_some(),
        "the first visit to an operation must hydrate"
    );
    seen.insert(a.to_string());

    // 2. Switch to B: it hydrates (nothing saved), and B is latched.
    assert_eq!(hydration_step(&batch_b, b, &seen), Some(None));
    seen.insert(b.to_string());

    // 3. Back to A, now with unsaved edits on the card. The old single-slot latch held only
    //    B here, so this returned `Some(..)` and clobbered them.
    assert_eq!(
        hydration_step(&batch_a, a, &seen),
        None,
        "returning to an already-hydrated operation must not re-apply its saved solution —              that silently discards in-progress edits"
    );
}

/// The weapon list duplicates `api/src/services/mortar.rs::charges_for`. Nothing can check
/// that from here, so this pins the copy: it fails the moment someone edits the list without
/// reading the note above it, which is the only warning this drift can get.
#[test]
fn the_offered_weapons_are_the_keys_the_api_accepts() {
    assert_eq!(
        WEAPONS,
        ["M252 81mm", "M821 81mm", "2B14 82mm", "M120 120mm"],
        "mirror of services/mortar.rs charges_for — update both or neither"
    );
    assert!(WEAPONS.iter().all(|w| *w == w.trim() && !w.is_empty()));
}

/// The `/events` rows this page decodes. `name_override` is genuinely optional on the model;
/// nothing else is, and the picker must not silently drop an operation it failed to read.
#[test]
fn the_operation_picker_decodes_a_real_events_row() {
    let page: Paginated<EventOption> = serde_json::from_str(
        r#"{"data":[{"id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","name_override":"Operation Byte Parity Night","start_time":"2026-08-01T19:00:00Z","status":"scheduled","registration_locked":false,"max_slots":0,"mission_count":1,"registered":5,"filled":5,"total_slots":16,"percent":31}],"total":1,"limit":50,"offset":0}"#,
    )
    .expect("a live /events row decodes");
    assert_eq!(page.data[0].id, "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7");
    assert_eq!(page.data[0].name(), "Operation Byte Parity Night");
    for blank in ["", r#","name_override":null"#, r#","name_override":"  ""#] {
        let anon: EventOption = serde_json::from_str(&format!(
            r#"{{"id":"x","start_time":"2026-08-01T19:00:00Z"{blank}}}"#
        ))
        .unwrap();
        assert_eq!(anon.name(), "Untitled Operation", "over {blank:?}");
    }
}
