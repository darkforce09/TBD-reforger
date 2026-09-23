//! The fleet scenario registry's checks, held against the backend's rules and the captured
//! registry, and the wiring of its two changes.

use super::scenario_wording::*;
use crate::v2::core::api::dto::{FleetScenarioList, FleetScenarioUpdate};
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use crate::v2::core::test_support::fixtures::golden;

/// Every captured mapping passes the checks a registration of it would meet.
#[test]
fn every_captured_mapping_passes_the_checks() {
    let registry: FleetScenarioList =
        serde_json::from_str(golden!("GET__fleet__scenarios.json")).unwrap();
    for scenario in &registry.items {
        assert_eq!(
            scenario_registration(
                &scenario.terrain_key,
                &scenario.scenario_id,
                &scenario.display_name
            ),
            Ok((
                scenario.terrain_key.clone(),
                FleetScenarioUpdate {
                    scenario_id: scenario.scenario_id.clone(),
                    display_name: scenario.display_name.clone(),
                }
            ))
        );
    }
    assert_eq!(
        updated_line(&registry.items[0], Some("000000000000000001")),
        "Updated by you, 2026-07-24 09:00 UTC"
    );
    assert_eq!(
        updated_line(&registry.items[0], None),
        "Updated by 000000000000000001, 2026-07-24 09:00 UTC"
    );
}

/// A terrain key is what the compiler writes: lowercase, starting with a letter, at most 64 bytes.
#[test]
fn terrain_keys_are_checked_as_the_backend_checks_them() {
    assert_eq!(validated_terrain_key(" everon "), Ok("everon".to_string()));
    assert!(validated_terrain_key("kunar_valley_2").is_ok());
    for bad in ["", "Everon", "2everon", "ever on", "ever-on", "évron"] {
        assert!(
            validated_terrain_key(bad).is_err(),
            "{bad:?} must be refused"
        );
    }
    assert!(validated_terrain_key(&format!("a{}", "b".repeat(63))).is_ok());
    assert!(validated_terrain_key(&format!("a{}", "b".repeat(64))).is_err());
}

/// A scenario id is a scenario header resource, exactly as the contract's pattern writes it.
#[test]
fn scenario_ids_follow_the_contract_pattern() {
    assert!(validated_scenario_id("{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf").is_ok());
    assert_eq!(
        validated_scenario_id("  {1111222233334444}Missions/TBD_Arland.conf "),
        Ok("{1111222233334444}Missions/TBD_Arland.conf".to_string())
    );
    for bad in [
        "Missions/TBD_Arland.conf",
        "{111122223333444}Missions/TBD_Arland.conf",
        "{1111222233334444a}Missions/TBD_Arland.conf",
        "{111122223333444a}Missions/TBD_Arland.conf",
        "{1111222233334444}.conf",
        "{1111222233334444}Missions/TBD Arland.conf",
        "{1111222233334444}Missions/TBD_Arland.ent",
    ] {
        assert!(
            validated_scenario_id(bad).is_err(),
            "{bad:?} must be refused"
        );
    }
}

/// A display name is trimmed and 1 to 128 bytes.
#[test]
fn display_names_are_trimmed_and_bounded() {
    assert_eq!(
        validated_display_name("  Arland  "),
        Ok("Arland".to_string())
    );
    assert!(validated_display_name("   ").is_err());
    assert!(validated_display_name(&"x".repeat(128)).is_ok());
    assert!(
        validated_display_name(&"é".repeat(65)).is_err(),
        "130 bytes"
    );
}

/// The two changes go through the typed endpoints, and the registry is read again after each.
#[test]
fn changes_go_through_the_typed_endpoints() {
    let src = live_code(include_str!("../mod.rs"));
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .replace(",)", ")")
    };
    let put = compact(only_body(&src, "pub(super) fn put("));
    assert!(put.contains("put_fleet_scenario(self.store,&terrain_key,&update)"));
    assert!(put.contains("self.reload()"));
    let remove = compact(only_body(&src, "pub(super) fn remove("));
    assert!(remove.contains("delete_fleet_scenario(self.store,&terrain_key)"));
    assert!(remove.contains("self.reload()"));
    let sheet = compact(&live_code(include_str!("../scenario_sheet.rs")));
    assert!(sheet.contains("scenario_registration("));
}
