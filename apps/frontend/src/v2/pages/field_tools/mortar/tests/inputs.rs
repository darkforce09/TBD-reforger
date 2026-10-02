//! Unit tests for the mortar calculator's inputs and catalog source: grid references and
//! heights, the terrain rule, the weapon/shell/charge choice, the wind, the battery, the burst
//! height, and the catalog list, identity check and offline wording.

use super::battery::{
    add_gun, default_battery, next_gun_label, remove_gun, resolve_battery, BatteryError, GunDraft,
    MAX_GUNS,
};
use super::illumination::{
    fuze_window_text, parse_burst_height, selected_time_fuze, BurstHeightError,
};
use super::positions::{
    effective_height_choice, grid_preview, resolve_position, HeightChoice, MortarTerrain,
    PositionDraft, PositionError,
};
use super::weapon_and_shell::{
    charge_from_value, charge_options, charge_value, reconcile_selection, shell_options,
    weapon_options, ArmamentSelection, ChargeChoice,
};
use super::wind::{parse_wind, WindDraft, WindInputError};
use crate::v2::core::api::dto::ballistics_catalogs::{
    BallisticsCatalog, BallisticsCatalogList, BallisticsCatalogSummary,
};
use crate::v2::core::offline::saved_copies::ReadSource;
use crate::v2::core::offline::{OfflineState, OfflineStatus};
use crate::v2::pages::field_tools::mortar::catalog_source::{
    check_document_matches, choose_catalog, failure_message, latest_catalog_versions,
    origin_notice, origin_of_read, CatalogFailure, CatalogKey, CatalogOrigin,
};
use map_coordinates::grid_reference::GridParseError;
use map_engine::data::scenario::ballistics::fire_mission::{FireMissionWind as Wind, HeightSource};

fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("mortar_test_catalog.json"))
        .expect("the test catalog decodes")
}

fn draft(grid: &str, height_choice: HeightChoice, manual_height: &str) -> PositionDraft {
    PositionDraft {
        grid: grid.to_string(),
        height_choice,
        manual_height: manual_height.to_string(),
    }
}

fn no_terrain_height(_: f64, _: f64) -> Option<f64> {
    None
}

#[test]
fn grids_of_six_eight_and_ten_figures_resolve_to_their_cell_centres() {
    let flat = |_: f64, _: f64| Some(12.5);
    for (grid, x, y) in [
        ("064 129", 6_450.0, 12_950.0),
        ("064129", 6_450.0, 12_950.0),
        ("0640 1290", 6_405.0, 12_905.0),
        ("06400 12900", 6_400.5, 12_900.5),
        ("  0640012900  ", 6_400.5, 12_900.5),
    ] {
        let point = resolve_position(
            &draft(grid, HeightChoice::Terrain, ""),
            MortarTerrain::Everon,
            flat,
        )
        .unwrap_or_else(|e| panic!("{grid:?} must resolve: {e:?}"));
        assert_eq!((point.x, point.y), (x, y), "{grid:?}");
        assert_eq!(point.height_m, 12.5);
        assert_eq!(point.height_source, HeightSource::Dem);
    }
}

#[test]
fn a_malformed_grid_is_refused_before_any_height_is_read() {
    let never = |_: f64, _: f64| -> Option<f64> { panic!("no height is read for a bad grid") };
    for (grid, expected) in [
        ("", GridParseError::Empty),
        ("06412", GridParseError::UnsupportedDigitCount(5)),
        (
            "064 1290",
            GridParseError::MismatchedHalves {
                easting_digits: 3,
                northing_digits: 4,
            },
        ),
        ("06a 129", GridParseError::InvalidCharacter('a')),
        ("064 129 1", GridParseError::TooManyGroups(3)),
    ] {
        assert_eq!(
            resolve_position(
                &draft(grid, HeightChoice::Terrain, ""),
                MortarTerrain::Everon,
                never
            ),
            Err(PositionError::Grid(expected)),
            "{grid:?}"
        );
    }
}

#[test]
fn a_missing_or_non_finite_terrain_height_is_an_error_never_a_zero() {
    for height_at in [no_terrain_height as fn(f64, f64) -> Option<f64>, |_, _| {
        Some(f64::NAN)
    }] {
        assert_eq!(
            resolve_position(
                &draft("064 129", HeightChoice::Terrain, "99"),
                MortarTerrain::Everon,
                height_at
            ),
            Err(PositionError::TerrainHeightUnavailable)
        );
    }
}

#[test]
fn the_terrain_height_is_sampled_at_the_cell_centre() {
    let sampled = std::cell::Cell::new(None);
    let point = resolve_position(
        &draft("0640 1290", HeightChoice::Terrain, ""),
        MortarTerrain::Everon,
        |x, y| {
            sampled.set(Some((x, y)));
            Some(87.25)
        },
    )
    .expect("resolves");
    assert_eq!(sampled.get(), Some((6_405.0, 12_905.0)));
    assert_eq!(point.height_m, 87.25);
}

#[test]
fn arland_only_resolves_manual_heights_and_never_reads_the_terrain() {
    assert!(!MortarTerrain::Arland.has_elevation_model());
    assert_eq!(
        effective_height_choice(MortarTerrain::Arland, HeightChoice::Terrain),
        HeightChoice::Manual
    );
    assert_eq!(
        effective_height_choice(MortarTerrain::Everon, HeightChoice::Terrain),
        HeightChoice::Terrain
    );
    let never = |_: f64, _: f64| -> Option<f64> { panic!("Arland has no elevation model") };
    let point = resolve_position(
        &draft("064 129", HeightChoice::Terrain, " 41.5 "),
        MortarTerrain::Arland,
        never,
    )
    .expect("a manual height resolves on Arland");
    assert_eq!(point.height_m, 41.5);
    assert_eq!(point.height_source, HeightSource::Manual);
    assert_eq!(
        resolve_position(
            &draft("064 129", HeightChoice::Terrain, ""),
            MortarTerrain::Arland,
            never
        ),
        Err(PositionError::ManualHeightMissing)
    );
}

#[test]
fn a_manual_height_must_be_a_finite_number() {
    for (text, expected) in [
        ("", Err(PositionError::ManualHeightMissing)),
        ("   ", Err(PositionError::ManualHeightMissing)),
        ("abc", Err(PositionError::ManualHeightInvalid("abc".into()))),
        ("inf", Err(PositionError::ManualHeightInvalid("inf".into()))),
        ("NaN", Err(PositionError::ManualHeightInvalid("NaN".into()))),
        ("-12.5", Ok(-12.5)),
    ] {
        let resolved = resolve_position(
            &draft("064 129", HeightChoice::Manual, text),
            MortarTerrain::Everon,
            no_terrain_height,
        )
        .map(|p| p.height_m);
        assert_eq!(resolved, expected, "{text:?}");
    }
}

#[test]
fn terrain_identifiers_round_trip_and_the_grid_preview_names_the_cell_centre() {
    for terrain in MortarTerrain::ALL {
        assert_eq!(
            MortarTerrain::from_terrain_id(terrain.terrain_id()),
            Some(terrain)
        );
    }
    assert_eq!(MortarTerrain::from_terrain_id("malden"), None);
    assert_eq!(grid_preview("064 129"), "x 6450.0 m, y 12950.0 m");
    assert!(grid_preview("").contains("6, 8 or 10 figures"));
    assert!(grid_preview("12345").contains("expected 6, 8 or 10"));
}

#[test]
fn weapons_are_labelled_with_their_mils_and_offer_only_the_shells_the_catalog_defines() {
    let catalog = catalog();
    let weapons: Vec<String> = weapon_options(&catalog)
        .into_iter()
        .map(|o| o.label)
        .collect();
    assert_eq!(weapons, ["M252 81mm (6400 mils)", "2B14 82mm (6000 mils)"]);
    let shells: Vec<String> = shell_options(&catalog, "m252")
        .into_iter()
        .map(|o| o.value)
        .collect();
    assert_eq!(
        shells,
        ["m821", "m853a1"],
        "a listed shell the catalog lacks is not offered"
    );
    let shells: Vec<String> = shell_options(&catalog, "2b14")
        .into_iter()
        .map(|o| o.value)
        .collect();
    assert_eq!(shells, ["o-832du"]);
    assert!(shell_options(&catalog, "m120").is_empty());
}

#[test]
fn every_charge_of_the_shell_is_offered_after_the_recommendation() {
    let catalog = catalog();
    let options = charge_options(&catalog, "m821");
    let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
    assert_eq!(values, ["recommended", "0", "1", "2", "3", "4"]);
    assert_eq!(options[3].label, "Charge 2 (game default)");
    assert_eq!(options[1].label, "Charge 0");
    let values: Vec<String> = charge_options(&catalog, "m853a1")
        .into_iter()
        .map(|o| o.value)
        .collect();
    assert_eq!(values, ["recommended", "1", "2", "3", "4"]);
    for choice in [
        ChargeChoice::Recommended,
        ChargeChoice::Rings(0),
        ChargeChoice::Rings(4),
    ] {
        assert_eq!(charge_from_value(&charge_value(choice)), choice);
    }
    assert_eq!(charge_from_value("junk"), ChargeChoice::Recommended);
}

#[test]
fn a_selection_is_reconciled_onto_the_catalog() {
    let catalog = catalog();
    let pick = |weapon: &str, shell: &str, charge| ArmamentSelection {
        weapon_id: weapon.into(),
        shell_id: shell.into(),
        charge,
    };
    assert_eq!(
        reconcile_selection(&catalog, &ArmamentSelection::default()),
        pick("m252", "m821", ChargeChoice::Recommended)
    );
    assert_eq!(
        reconcile_selection(&catalog, &pick("2b14", "m821", ChargeChoice::Rings(4))),
        pick("2b14", "o-832du", ChargeChoice::Rings(4)),
        "a shell the weapon does not fire falls back to its first shell; a charge it has stays"
    );
    assert_eq!(
        reconcile_selection(&catalog, &pick("m252", "m853a1", ChargeChoice::Rings(0))),
        pick("m252", "m853a1", ChargeChoice::Recommended),
        "M853A1 has no charge 0"
    );
    assert_eq!(
        reconcile_selection(&catalog, &pick("m120", "m821", ChargeChoice::Rings(2))),
        pick("m252", "m821", ChargeChoice::Rings(2))
    );
    let mut empty = catalog.clone();
    empty.weapons.clear();
    assert_eq!(
        reconcile_selection(&empty, &pick("m252", "m821", ChargeChoice::Rings(2))),
        ArmamentSelection::default()
    );
}

fn wind(speed: &str, from: &str) -> Result<Option<Wind>, WindInputError> {
    parse_wind(&WindDraft {
        speed_m_s: speed.into(),
        from_deg: from.into(),
    })
}

#[test]
fn the_wind_parses_speed_and_the_direction_it_blows_from() {
    assert_eq!(wind("", ""), Ok(None));
    assert_eq!(wind("0", ""), Ok(None));
    assert_eq!(wind(" 0 ", "270"), Ok(None));
    assert_eq!(
        wind("4.5", "270"),
        Ok(Some(Wind {
            speed_m_s: 4.5,
            from_deg: 270.0
        }))
    );
    let normalised = |from: &str| wind("3", from).unwrap().unwrap().from_deg;
    assert_eq!(normalised("-90"), 270.0);
    assert_eq!(normalised("360"), 0.0);
    assert_eq!(normalised("725"), 5.0);
}

#[test]
fn an_unreadable_or_incomplete_wind_is_refused() {
    assert_eq!(
        wind("-1", "90"),
        Err(WindInputError::InvalidSpeed("-1".into()))
    );
    assert_eq!(
        wind("fast", "90"),
        Err(WindInputError::InvalidSpeed("fast".into()))
    );
    assert_eq!(
        wind("inf", "90"),
        Err(WindInputError::InvalidSpeed("inf".into()))
    );
    assert_eq!(wind("3", ""), Err(WindInputError::DirectionMissing));
    assert_eq!(wind("", "90"), Err(WindInputError::SpeedMissing));
    assert_eq!(
        wind("3", "west"),
        Err(WindInputError::InvalidDirection("west".into()))
    );
    assert_eq!(
        wind("3", "NaN"),
        Err(WindInputError::InvalidDirection("NaN".into()))
    );
}

#[test]
fn guns_are_added_with_the_lowest_free_label_up_to_the_limit() {
    let mut guns = default_battery();
    assert_eq!(guns.len(), 1);
    assert_eq!(guns[0].label, "Gun 1");
    guns[0].position.height_choice = HeightChoice::Manual;
    assert!(add_gun(&mut guns));
    assert_eq!(guns[1].label, "Gun 2");
    assert_eq!(
        guns[1].position.height_choice,
        HeightChoice::Manual,
        "inherits the height source"
    );
    assert!(guns[1].position.grid.is_empty());
    guns[0].label = "Alpha".into();
    assert_eq!(next_gun_label(&guns), "Gun 1");
    while guns.len() < MAX_GUNS {
        assert!(add_gun(&mut guns));
    }
    assert!(!add_gun(&mut guns), "a full battery refuses another gun");
    assert_eq!(guns.len(), MAX_GUNS);
    let mut keys: Vec<u32> = guns.iter().map(|g| g.key).collect();
    keys.dedup();
    assert_eq!(keys.len(), MAX_GUNS, "keys are unique");
}

/// The contract every saved fire mission is checked against.
const FIRE_MISSION_SCHEMA: &str =
    include_str!("../../../../../../../../contracts/definitions/fire-mission.schema.json");

#[test]
fn the_battery_cap_is_the_contract_cap_on_saved_guns() {
    let schema: serde_json::Value =
        serde_json::from_str(FIRE_MISSION_SCHEMA).expect("the contract is JSON");
    let contract_cap = schema["definitions"]["FireMissionSave"]["properties"]["guns"]["maxItems"]
        .as_u64()
        .expect("FireMissionSave.guns declares maxItems");
    assert_eq!(MAX_GUNS as u64, contract_cap);
    assert_eq!(MAX_GUNS, 12);
}

#[test]
fn the_last_gun_cannot_be_removed_and_keys_stay_unique() {
    let mut guns = default_battery();
    assert!(!remove_gun(&mut guns, 0));
    add_gun(&mut guns);
    add_gun(&mut guns);
    assert!(!remove_gun(&mut guns, 99), "an unknown key removes nothing");
    assert!(remove_gun(&mut guns, 1));
    add_gun(&mut guns);
    let keys: Vec<u32> = guns.iter().map(|g| g.key).collect();
    assert_eq!(keys, [0, 2, 3]);
    let labels: Vec<&str> = guns.iter().map(|g| g.label.as_str()).collect();
    assert_eq!(labels, ["Gun 1", "Gun 3", "Gun 2"]);
}

fn gun(key: u32, label: &str, grid: &str, height: &str) -> GunDraft {
    GunDraft {
        key,
        label: label.into(),
        position: draft(grid, HeightChoice::Manual, height),
    }
}

#[test]
fn a_battery_resolves_in_order_or_reports_every_problem() {
    let ok = [
        gun(0, " Gun 1 ", "064 129", "10"),
        gun(1, "Gun 2", "065 129", "12"),
    ];
    let resolved = resolve_battery(&ok, MortarTerrain::Everon, no_terrain_height).expect("valid");
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].label, "Gun 1", "labels are trimmed");
    assert_eq!((resolved[1].x, resolved[1].height_m), (6_550.0, 12.0));
    assert_eq!(resolved[1].height_source, HeightSource::Manual);

    let bad = [
        gun(0, "Gun 1", "064 129", "10"),
        gun(1, "gun 1", "bad", "10"),
        gun(2, "Gun 1", "064 129", "x"),
        gun(3, "  ", "064 129", "3"),
    ];
    let errors = resolve_battery(&bad, MortarTerrain::Everon, no_terrain_height).unwrap_err();
    assert_eq!(
        errors,
        [
            BatteryError::Position {
                label: "gun 1".into(),
                error: PositionError::Grid(GridParseError::InvalidCharacter('b')),
            },
            BatteryError::DuplicateLabel("Gun 1".into()),
            BatteryError::Position {
                label: "Gun 1".into(),
                error: PositionError::ManualHeightInvalid("x".into()),
            },
            BatteryError::EmptyLabel { index: 3 },
        ]
    );
}

#[test]
fn only_a_time_fuzed_shell_takes_a_burst_height() {
    let catalog = catalog();
    let selected = |shell: &str| ArmamentSelection {
        weapon_id: "m252".into(),
        shell_id: shell.into(),
        charge: ChargeChoice::Recommended,
    };
    assert!(selected_time_fuze(&catalog, &selected("m821")).is_none());
    let fuze = selected_time_fuze(&catalog, &selected("m853a1")).expect("M853A1 is time-fuzed");
    assert_eq!(fuze_window_text(&fuze), "Fuze 10–40 s, default 24 s");
    assert_eq!(parse_burst_height("garbage", None), Ok(None));
    assert_eq!(parse_burst_height("", Some(&fuze)), Ok(None));
    assert_eq!(parse_burst_height(" 600 ", Some(&fuze)), Ok(Some(600.0)));
    for text in ["0", "-5", "abc", "inf"] {
        assert_eq!(
            parse_burst_height(text, Some(&fuze)),
            Err(BurstHeightError::Invalid(text.into())),
            "{text:?}"
        );
    }
}

fn summary(id: &str, version: u32) -> BallisticsCatalogSummary {
    BallisticsCatalogSummary {
        catalog_id: id.into(),
        catalog_version: version,
        title: format!("{id} v{version}"),
        game_build: "1.8.0.13".into(),
        export_generation_id: "6A6F008DC5395616".into(),
        catalog_sha256: "0".repeat(64),
        uploaded_at: "2026-09-28T00:00:00Z".into(),
    }
}

#[test]
fn the_newest_version_of_each_catalog_is_offered_and_a_valid_choice_is_kept() {
    let list = BallisticsCatalogList {
        data: vec![
            summary("vanilla", 1),
            summary("modded", 4),
            summary("vanilla", 3),
            summary("vanilla", 2),
        ],
    };
    let latest = latest_catalog_versions(&list);
    let keys: Vec<CatalogKey> = latest.iter().map(CatalogKey::of_summary).collect();
    assert_eq!(
        keys,
        [
            CatalogKey {
                catalog_id: "modded".into(),
                catalog_version: 4
            },
            CatalogKey {
                catalog_id: "vanilla".into(),
                catalog_version: 3
            },
        ]
    );
    assert_eq!(choose_catalog(&latest, None), Some(keys[0].clone()));
    assert_eq!(
        choose_catalog(&latest, Some(&keys[1])),
        Some(keys[1].clone())
    );
    let stale = CatalogKey {
        catalog_id: "vanilla".into(),
        catalog_version: 2,
    };
    assert_eq!(choose_catalog(&latest, Some(&stale)), Some(keys[0].clone()));
    assert_eq!(choose_catalog(&[], Some(&stale)), None);
    assert_eq!(
        keys[1].document_path(),
        "/ballistics-catalogs/vanilla/versions/3"
    );
}

#[test]
fn a_document_for_another_version_is_refused() {
    let catalog = catalog();
    let asked = CatalogKey {
        catalog_id: "mortar-page-test".into(),
        catalog_version: 2,
    };
    assert_eq!(check_document_matches(&asked, &catalog), Ok(()));
    for other in [
        CatalogKey {
            catalog_id: "mortar-page-test".into(),
            catalog_version: 1,
        },
        CatalogKey {
            catalog_id: "vanilla".into(),
            catalog_version: 2,
        },
    ] {
        let refusal = check_document_matches(&other, &catalog).unwrap_err();
        assert!(refusal.contains("mortar-page-test v2"), "{refusal}");
    }
}

#[test]
fn the_catalog_origin_and_failures_are_worded_from_the_offline_state() {
    assert_eq!(origin_of_read(ReadSource::Server), CatalogOrigin::Network);
    let saved = CatalogOrigin::OfflineCopy {
        saved_on: Some("28 Sep 2026, 14:05 UTC".into()),
    };
    assert_eq!(
        origin_of_read(ReadSource::SavedCopy {
            saved_on: Some("28 Sep 2026, 14:05 UTC".into())
        }),
        saved
    );
    assert_eq!(origin_notice(CatalogOrigin::Network), None);
    assert!(origin_notice(saved)
        .unwrap()
        .starts_with("Offline copy from 28 Sep 2026, 14:05 UTC"));
    let status = |state, progress_percent| OfflineStatus {
        state,
        progress_percent,
    };
    let unreadable = CatalogFailure::ListUnreadable("network error".into());
    for (state, needle) in [
        (OfflineState::Idle, "No offline copy"),
        (OfflineState::Failed, "No offline copy"),
        (OfflineState::Downloading, "still downloading (40%)"),
        (OfflineState::Ready, "holds no readable catalog"),
        (OfflineState::Incomplete, "holds no readable catalog"),
        (OfflineState::QuotaShort, "free storage"),
        (OfflineState::Unsupported, "cannot keep an offline copy"),
    ] {
        let message = failure_message(&unreadable, status(state, 40));
        assert!(message.starts_with("The ballistics catalogs could not be loaded (network error)."));
        assert!(message.contains(needle), "{state:?}: {message}");
    }
    let none = failure_message(
        &CatalogFailure::NoCatalogs,
        status(OfflineState::Ready, 100),
    );
    assert!(none.starts_with("No ballistics catalog has been published yet"));
    let doc = failure_message(
        &CatalogFailure::DocumentUnreadable("404".into()),
        status(OfflineState::Idle, 0),
    );
    assert!(doc.starts_with("The chosen ballistics catalog could not be loaded (404)."));
}
