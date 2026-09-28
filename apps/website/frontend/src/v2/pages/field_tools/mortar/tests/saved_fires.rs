//! The guards on saving and restoring fire missions: the save body, the legacy and catalog-model
//! rows, the hydration latch, the event picker and the save answer.

use super::list::saved_row_lines;
use super::restore::{gun_drafts, parse_legacy_grid, position_draft, restore, RestoredPosition};
use super::save_request::{save_body, save_refusal_text, save_status_text, SaveStatus};
use super::*;
use crate::v2::core::api::dto::{DataEnvelope, HeightSource, Paginated};
use crate::v2::pages::field_tools::mortar::inputs::positions::{HeightChoice, PositionDraft};
use crate::v2::pages::field_tools::mortar::inputs::weapon_and_shell::ChargeChoice;
use crate::v2::pages::field_tools::mortar::test_mission::{gun, manual, solved_mission};

/// A row of `GET /events/{id}/fire-missions` stored before the coordinate columns: FP
/// (1000, 2000) → TGT (2200, 1800) as legacy `x, y` grid text, `M252 81mm`.
const LIVE_LIST: &str = r#"{"data":[{"id":"97176662-5589-4831-85e5-61f2a7bd8597","event_id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","created_by":"000000000000000001","weapon_system":"M252 81mm","fp_grid":"1000, 2000","target_grid":"2200, 1800","distance_m":1217,"azimuth_deg":99.5,"elevation_mils":1315,"created_at":"2026-07-31T02:16:52.695935Z"}]}"#;

/// The same fire mission stored once the coordinate, charge and time-of-flight columns existed.
const LIVE_LIST_WITH_COLUMNS: &str = r#"{"data":[{"id":"5c2e8b4a-1f77-4a10-9d3e-2b6c7f0a1e44","event_id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","created_by":"000000000000000001","weapon_system":"M252 81mm","fp_grid":"1000, 2000","target_grid":"2200, 1800","distance_m":1217,"azimuth_deg":99.5,"elevation_mils":1315,"fp_x":1000.0,"fp_y":2000.0,"tgt_x":2200.0,"tgt_y":1800.0,"azimuth_mils":1768,"charge":2,"time_of_flight_s":29.4,"created_at":"2026-08-01T10:04:11.512004Z"}]}"#;

const EVENT: &str = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";

fn legacy_row() -> SavedFire {
    serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
        .expect("the legacy list decodes")
        .data
        .remove(0)
}

/// A catalog-model row as the list returns it after a save of [`solved_mission`].
fn catalog_row() -> SavedFire {
    serde_json::from_value(serde_json::json!({
        "id": "0b8e2f7c-3d41-4c55-9a0e-6f1d2c3b4a59", "event_id": EVENT,
        "created_by": "000000000000000001", "weapon_system": "m252",
        "fp_grid": "05500 12500", "target_grid": "064 129", "distance_m": 985,
        "azimuth_deg": 60.0, "elevation_mils": 1100, "tgt_x": 6450.0, "tgt_y": 12950.0,
        "catalog_id": "vanilla-mortars", "catalog_version": 1, "weapon_id": "m252",
        "shell_id": "m853a1", "charge_rings": 2, "target_height_m": 40.0,
        "target_height_source": "manual", "wind_speed_m_s": 4.0, "wind_from_deg": 200.0,
        "burst_height_m": 500.0,
        "guns": [
            {"gun_index": 0, "label": "Gun 1", "x": 5550.0, "y": 12550.0, "height_m": 25.0,
             "height_source": "manual", "azimuth_mils": 1100.0, "elevation_mils": 1200.0,
             "charge_rings": 2, "time_of_flight_s": 30.0},
            {"gun_index": 1, "label": "Gun 2", "x": 5650.0, "y": 12450.0, "height_m": 31.5,
             "height_source": "dem", "azimuth_mils": 1090.0, "elevation_mils": null,
             "charge_rings": null, "time_of_flight_s": null}
        ],
        "created_at": "2026-09-28T10:00:00Z"
    }))
    .expect("a catalog-model row decodes")
}

#[test]
fn a_row_saved_before_the_coordinate_columns_restores_through_its_legacy_grid_text() {
    let row = legacy_row();
    assert_eq!(
        (row.fp_x, row.fp_y, row.tgt_x, row.tgt_y),
        (None, None, None, None)
    );
    let r = restore(&row).expect("a legacy row restores via its grid text");
    assert_eq!((r.target.x, r.target.y), (2200.0, 1800.0));
    assert_eq!(r.guns.len(), 1, "a legacy row has one fire position");
    assert_eq!(r.guns[0].0, "Gun 1");
    assert_eq!((r.guns[0].1.x, r.guns[0].1.y), (1000.0, 2000.0));
    assert_eq!(r.target.height, None, "a legacy row records no height");
    assert_eq!(
        r.selection, None,
        "a legacy weapon name is no catalog selection"
    );
    assert_eq!(r.saved_at, "2026-07-31T02:16:52.695935Z");
}

#[test]
fn a_row_with_coordinate_columns_restores_without_its_grid_text() {
    let row = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST_WITH_COLUMNS)
        .unwrap()
        .data
        .remove(0);
    let r = restore(&row).expect("a row with columns restores");
    let mut no_grid = row.clone();
    no_grid.fp_grid = "GRID ONE".into();
    no_grid.target_grid = "GRID TWO".into();
    let r2 = restore(&no_grid).expect("the numeric columns carry it without the grid text");
    assert_eq!(r2.target, r.target);
    assert_eq!(r2.guns, r.guns);
    assert_eq!((r.target.x, r.target.y), (2200.0, 1800.0));
    assert_eq!((r.guns[0].1.x, r.guns[0].1.y), (1000.0, 2000.0));
    assert_eq!(
        row.azimuth_mils,
        Some(1768),
        "the sight setting is on the row"
    );
    assert_eq!((row.charge, row.time_of_flight_s), (Some(2), Some(29.4)));
}

/// The legacy grid text is the only record of an old row's coordinates, so it reads back every
/// value the old inputs could hold: negatives, fractions, zero and far off any terrain.
#[test]
fn every_legacy_grid_text_reads_back_its_two_coordinates() {
    for (x, y) in [
        (0.0, 0.0),
        (1000.0, 2000.0),
        (-5000.5, 9000.25),
        (12_800.0, 0.125),
        (1e7, -1e7),
    ] {
        assert_eq!(parse_legacy_grid(&format!("{x}, {y}")), Some((x, y)));
        assert_eq!(parse_legacy_grid(&format!("  {x} ,{y}  ")), Some((x, y)));
    }
}

/// Text that is neither a grid reference nor the legacy encoding restores as nothing, never as
/// the origin; a grid reference restores to its cell centre.
#[test]
fn a_foreign_grid_text_restores_as_nothing_rather_than_as_the_origin() {
    for s in ["", "1000", "AB, CD", "1000, ", "NaN, 3", "inf, 2"] {
        assert_eq!(parse_legacy_grid(s), None, "{s:?} should not parse");
        let mut row = legacy_row();
        row.fp_grid = s.into();
        assert_eq!(restore(&row), None, "a gun at {s:?} must not restore");
    }
    let mut row = legacy_row();
    row.target_grid = "064 129".into();
    let r = restore(&row).expect("a grid reference restores");
    assert_eq!((r.target.x, r.target.y), (6450.0, 12950.0));
}

#[test]
fn a_catalog_model_row_restores_every_gun_its_heights_and_the_armament() {
    let r = restore(&catalog_row()).expect("a catalog-model row restores");
    assert_eq!(r.target.height, Some((40.0, HeightSource::Manual)));
    assert_eq!(r.guns.len(), 2);
    assert_eq!(r.guns[1].0, "Gun 2");
    assert_eq!(r.guns[1].1.height, Some((31.5, HeightSource::Dem)));
    let selection = r
        .selection
        .clone()
        .expect("the row names its weapon and shell");
    assert_eq!(
        (selection.weapon_id.as_str(), selection.shell_id.as_str()),
        ("m252", "m853a1")
    );
    assert_eq!(selection.charge, ChargeChoice::Rings(2));
    let wind = r.wind.clone().expect("the wind was recorded");
    assert_eq!(
        (wind.speed_m_s.as_str(), wind.from_deg.as_str()),
        ("4", "200")
    );
    assert_eq!(r.burst_height.as_deref(), Some("500"));

    let drafts = gun_drafts(&r, &[gun(7, "Old", "000 000", "12")]);
    assert_eq!(drafts.len(), 2);
    assert_eq!((drafts[0].key, drafts[1].key), (0, 1));
    assert_eq!(drafts[0].position.grid, "05550 12550");
    assert_eq!(drafts[0].position.height_choice, HeightChoice::Manual);
    assert_eq!(drafts[0].position.manual_height, "25");
    assert_eq!(drafts[1].position.height_choice, HeightChoice::Terrain);
}

#[test]
fn a_position_without_a_recorded_height_keeps_the_current_height_choice() {
    let fallback = manual("000 000", "17");
    let restored = RestoredPosition {
        x: 1000.0,
        y: 2000.0,
        height: None,
    };
    let draft = position_draft(&restored, &fallback);
    assert_eq!(draft.grid, "01000 02000");
    assert_eq!(draft.height_choice, HeightChoice::Manual);
    assert_eq!(draft.manual_height, "17");
    assert_eq!(
        position_draft(&restored, &PositionDraft::default()).height_choice,
        HeightChoice::Terrain
    );
}

/// The save body is the solve's own inputs and solution, pinned to the catalog version.
#[test]
fn the_save_body_carries_the_solved_inputs_and_the_client_solution() {
    let solved = solved_mission();
    let body = save_body(&solved, Some("  ev-1  "));
    assert_eq!(body.event_id.as_deref(), Some("ev-1"), "the id is trimmed");
    assert_eq!(
        (body.catalog_id.as_str(), body.catalog_version),
        (
            solved.inputs.catalog_id.as_str(),
            solved.inputs.catalog_version
        )
    );
    assert_eq!(
        (body.weapon_id.as_str(), body.shell_id.as_str()),
        ("m252", "m853a1")
    );
    assert_eq!(
        body.charge_rings, None,
        "a recommended charge is not pinned"
    );
    assert_eq!(body.target.height_m, 40.0);
    assert_eq!(body.target.height_source, HeightSource::Manual);
    assert_eq!(body.guns.len(), 2);
    assert_eq!(body.guns[1].label, "Gun 2");
    assert_eq!(
        (body.guns[1].x, body.guns[1].y),
        (solved.inputs.guns[1].x, solved.inputs.guns[1].y)
    );
    let wind = body.wind.as_ref().expect("the wind is sent");
    assert_eq!((wind.speed_m_s, wind.from_deg), (4.0, 200.0));
    assert_eq!(body.burst_height_m, Some(500.0));
    assert_eq!(body.target_grid, "064 129");
    assert_eq!(
        body.client_solution, solved.solution,
        "the page's own solution is posted"
    );

    for none in [None, Some(""), Some("   ")] {
        let json = serde_json::to_value(save_body(&solved, none)).unwrap();
        assert!(
            json.get("event_id").is_none(),
            "a blank event_id is omitted: {json}"
        );
    }
}

/// What is saved comes back: the row the API stores from a save body restores the same
/// positions and heights the body carried.
#[test]
fn a_saved_body_restores_to_the_positions_it_carried() {
    let solved = solved_mission();
    let body = save_body(&solved, Some(EVENT));
    let mut row = catalog_row();
    row.tgt_x = Some(body.target.x);
    row.tgt_y = Some(body.target.y);
    row.target_height_m = Some(body.target.height_m);
    row.target_height_source = Some(body.target.height_source);
    for (stored, sent) in row.guns.iter_mut().zip(&body.guns) {
        (stored.x, stored.y, stored.height_m) = (sent.x, sent.y, sent.height_m);
        stored.height_source = sent.height_source;
    }
    let r = restore(&row).unwrap();
    assert_eq!((r.target.x, r.target.y), (body.target.x, body.target.y));
    for ((_, restored), sent) in r.guns.iter().zip(&body.guns) {
        assert_eq!(
            (restored.x, restored.y, restored.height),
            (sent.x, sent.y, Some((sent.height_m, sent.height_source)))
        );
    }
}

/// `LocalResource` serves the previous key's value while the next is in flight: a batch fetched
/// for another event must neither hydrate nor latch, and the real batch must still hydrate.
#[test]
fn hydration_refuses_a_batch_fetched_for_a_different_event() {
    let stale = SavedFor {
        event_id: None,
        rows: Vec::new(),
    };
    assert_eq!(hydration_step(&stale, EVENT, &HashSet::new()), None);
    let fresh = SavedFor {
        event_id: Some(EVENT.to_string()),
        rows: vec![legacy_row()],
    };
    let applied = hydration_step(&fresh, EVENT, &HashSet::new())
        .expect("the batch for this event is acted on")
        .expect("its newest row restores");
    assert_eq!((applied.guns[0].1.x, applied.guns[0].1.y), (1000.0, 2000.0));
    assert_eq!(
        hydration_step(&fresh, EVENT, &HashSet::from([EVENT.to_string()])),
        None,
        "a refetch after a save must not re-hydrate"
    );
    let empty = SavedFor {
        event_id: Some(EVENT.to_string()),
        rows: Vec::new(),
    };
    assert_eq!(hydration_step(&empty, EVENT, &HashSet::new()), Some(None));
}

/// Event A → B → A must not re-hydrate A over drafts edited since: the latch is a set.
#[test]
fn returning_to_an_event_does_not_re_hydrate_over_unsaved_edits() {
    let (a, b) = (EVENT, "00000000-0000-4000-7000-000000000001");
    let batch_a = SavedFor {
        event_id: Some(a.to_string()),
        rows: vec![legacy_row()],
    };
    let batch_b = SavedFor {
        event_id: Some(b.to_string()),
        rows: Vec::new(),
    };
    let mut seen: HashSet<String> = HashSet::new();
    assert!(hydration_step(&batch_a, a, &seen).is_some());
    seen.insert(a.to_string());
    assert_eq!(hydration_step(&batch_b, b, &seen), Some(None));
    seen.insert(b.to_string());
    assert_eq!(hydration_step(&batch_a, a, &seen), None);
}

#[test]
fn the_event_picker_decodes_a_real_events_row() {
    let page: Paginated<EventOption> = serde_json::from_str(
        r#"{"data":[{"id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","name_override":"Operation Byte Parity Night","start_time":"2026-08-01T19:00:00Z","status":"scheduled","registration_locked":false,"max_slots":0,"mission_count":1,"registered":5,"filled":5,"total_slots":16,"percent":31}],"total":1,"limit":50,"offset":0}"#,
    )
    .expect("a live /events row decodes");
    assert_eq!(page.data[0].id, EVENT);
    assert_eq!(page.data[0].name(), "Operation Byte Parity Night");
    for blank in ["", r#","name_override":null"#, r#","name_override":"  ""#] {
        let anon: EventOption = serde_json::from_str(&format!(
            r#"{{"id":"x","start_time":"2026-08-01T19:00:00Z"{blank}}}"#
        ))
        .unwrap();
        assert_eq!(anon.name(), "Untitled Operation", "over {blank:?}");
    }
}

#[test]
fn a_solution_mismatch_is_explained_and_other_refusals_are_named() {
    let mismatch = save_refusal_text(422, Some("mismatch"), Some("solution_mismatch"));
    assert!(mismatch.contains("differs"), "{mismatch}");
    assert!(save_refusal_text(404, None, None).contains("no longer exists"));
    assert!(save_refusal_text(0, None, None).contains("could not be reached"));
    let other = save_refusal_text(422, Some("target_grid is required"), Some("validation"));
    assert!(
        other.contains("422") && other.contains("target_grid is required"),
        "{other}"
    );
    assert_eq!(save_status_text(&SaveStatus::Idle), None);
    assert_eq!(
        save_status_text(&SaveStatus::Saved("2026-09-28T10:00:00Z".into())).as_deref(),
        Some("Saved (2026-09-28T10:00:00Z).")
    );
}

#[test]
fn the_saved_list_words_legacy_and_catalog_model_rows() {
    let (positions, figures) = saved_row_lines(&legacy_row());
    assert_eq!(positions, "1000, 2000 → 2200, 1800");
    assert_eq!(figures, "M252 81mm · 1217 m · 99.5° · 1315 mils");
    let (positions, figures) = saved_row_lines(&catalog_row());
    assert_eq!(positions, "2 guns → 064 129");
    assert_eq!(figures, "m252 · m853a1 · charge 2");
}

#[test]
fn an_offline_page_replaces_the_sign_in_prompt_with_the_needs_a_connection_notice() {
    use super::connection_gate::{save_area_access, SaveAreaAccess, NEEDS_CONNECTION_TEXT};
    use crate::v2::pages::field_tools::mortar::catalog_source::CatalogOrigin;
    for saved_on in [None, Some("28 Sep 2026".to_string())] {
        let offline = CatalogOrigin::OfflineCopy { saved_on };
        assert_eq!(
            save_area_access(Some(&offline)),
            SaveAreaAccess::NeedsConnection
        );
    }
    assert!(NEEDS_CONNECTION_TEXT.contains("need a connection"));
    assert!(!NEEDS_CONNECTION_TEXT.to_lowercase().contains("sign in"));
}

#[test]
fn an_online_or_unread_page_keeps_the_session_gate() {
    use super::connection_gate::{save_area_access, SaveAreaAccess};
    use crate::v2::pages::field_tools::mortar::catalog_source::CatalogOrigin;
    assert_eq!(
        save_area_access(Some(&CatalogOrigin::Network)),
        SaveAreaAccess::SessionGate
    );
    assert_eq!(save_area_access(None), SaveAreaAccess::SessionGate);
}
