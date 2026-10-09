use super::*;
use frontend_api_dtos::UserFaction;
use mission_operations::faction_library::{FactionDoc, FactionRole, FactionVehicle};

fn uf(id: &str, side: &str, name: &str) -> UserFaction {
    UserFaction {
        id: id.into(),
        owner_id: "o".into(),
        side: side.into(),
        name: name.into(),
        doc: FactionDoc {
            side: side.into(),
            name: name.into(),
            ..Default::default()
        },
        created_at: String::new(),
        updated_at: String::new(),
    }
}

/// H5 — Cancel path never allows apply.
#[test]
fn apply_cancel_noop() {
    assert!(!apply_confirm_allows(false));
    assert!(apply_confirm_allows(true));
}

/// H6 — Save inverse roles.len matches authored slot count.
#[test]
fn save_faction_roles_match_side() {
    let doc = FactionDoc {
        side: "BLUFOR".into(),
        name: "Alpha".into(),
        emblem: None,
        roles: vec![
            FactionRole {
                role: "SL".into(),
                tag: None,
                character: "c1".into(),
                loadout: None,
            },
            FactionRole {
                role: "Rifleman".into(),
                tag: None,
                character: "c2".into(),
                loadout: None,
            },
        ],
        vehicles: vec![FactionVehicle {
            vehicle: "v1".into(),
            label: None,
        }],
    };
    assert_eq!(faction_doc_role_count(&doc), 2);
    assert_eq!(doc.roles.len(), 2);
}

/// H7 / H-L8 — CIV + other sides excluded from dropdown.
#[test]
fn template_options_exclude_civ_and_other_sides() {
    let lib = vec![
        uf("1", "BLUFOR", "US 1980s"),
        uf("2", "OPFOR", "Soviet"),
        uf("3", "CIV", "Civilians"),
        uf("4", "INDFOR", "FIA"),
    ];
    let blu = template_options_for_side(&lib, "BLUFOR");
    assert_eq!(blu.len(), 1);
    assert_eq!(blu[0].id, "1");
    assert!(blu.iter().all(|f| f.side != "CIV"));
    let opf = template_options_for_side(&lib, "OPFOR");
    assert_eq!(opf.len(), 1);
    assert_eq!(opf[0].name, "Soviet");
    let civ_tab = template_options_for_side(&lib, "CIV");
    assert!(civ_tab.is_empty(), "CIV never a template side");
}

// ---- T-373: Save-from-side must not destroy what the ORBAT cannot express ----

/// The authoring the mission graph has nowhere to store, marked so a loss is visible in a
/// stored document rather than inferred from an absence.
const SENTINEL_EMBLEM: &str = "SENTINEL emblem [T-373]";
const SENTINEL_LABEL_A: &str = "SENTINEL Alpha 1-1 [T-373]";
const SENTINEL_LABEL_B: &str = "SENTINEL Alpha 1-2 [T-373]";
const M151: &str = "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et";
const UAZ: &str = "{AAAAAAAAAAAAAAAA}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et";
const CHAR: &str = "{BBBBBBBBBBBBBBBB}Prefabs/Characters/Factions/US/US_Rifleman.et";

fn role(name: &str, loadout: Option<serde_json::Value>) -> FactionRole {
    FactionRole {
        role: name.into(),
        tag: None,
        character: CHAR.into(),
        loadout,
    }
}

fn veh(resource: &str, label: Option<&str>) -> FactionVehicle {
    FactionVehicle {
        vehicle: resource.into(),
        label: label.map(str::to_string),
    }
}

fn loadout() -> serde_json::Value {
    serde_json::json!({
        "version": 2,
        "wear": { "jacket": "{CCCCCCCCCCCCCCCC}Prefabs/Clothing/Jacket.et" },
        "weapons": [],
        "summary": "SENTINEL loadout [T-373]"
    })
}

/// The library entry as the operator authored it: an emblem plus two labelled M151s.
fn stored_template() -> FactionDoc {
    FactionDoc {
        side: "BLUFOR".into(),
        name: "US Army 1980s".into(),
        emblem: Some(SENTINEL_EMBLEM.into()),
        roles: vec![
            role("Squad Leader", Some(loadout())),
            role("Rifleman", None),
        ],
        vehicles: vec![
            veh(M151, Some(SENTINEL_LABEL_A)),
            veh(M151, Some(SENTINEL_LABEL_B)),
        ],
    }
}

/// What `engine_ops::faction_doc_from_side` can see: no emblem, no labels — ever.
fn derived_from_side() -> FactionDoc {
    FactionDoc {
        side: "BLUFOR".into(),
        name: "BLUFOR".into(),
        emblem: None,
        roles: vec![
            role("Squad Leader", Some(loadout())),
            role("Rifleman", None),
        ],
        vehicles: vec![veh(M151, None), veh(M151, None)],
    }
}

/// The defect itself, pinned: PUTting the derivation raw omits both keys, and a whole-document
/// replace reads an omitted key as a deletion. If this ever starts carrying them, the merge
/// below has become redundant — which is a fine thing to be told, loudly.
#[test]
fn t373_derived_body_alone_omits_emblem_and_labels() {
    let raw = serde_json::to_string(&derived_from_side()).expect("serialize");
    assert!(
        !raw.contains("emblem"),
        "skip_serializing_if drops the key entirely: {raw}"
    );
    assert!(
        !raw.contains("label"),
        "and every vehicle label with it: {raw}"
    );
}

#[test]
fn t373_merge_preserves_the_emblem_the_orbat_cannot_express() {
    let stored = stored_template();
    let merged =
        merge_faction_doc_from_side(&stored, derived_from_side()).expect("content-bearing side");
    assert_eq!(merged.emblem.as_deref(), Some(SENTINEL_EMBLEM));
    let raw = serde_json::to_string(&merged).expect("serialize");
    assert!(
        raw.contains(SENTINEL_EMBLEM),
        "the emblem must reach the wire, not just the struct: {raw}"
    );
}

#[test]
fn t373_merge_repairs_vehicle_labels_by_resource_in_order() {
    let stored = stored_template();
    let merged = merge_faction_doc_from_side(&stored, derived_from_side()).expect("ok");
    assert_eq!(merged.vehicles.len(), 2);
    assert_eq!(merged.vehicles[0].label.as_deref(), Some(SENTINEL_LABEL_A));
    assert_eq!(
        merged.vehicles[1].label.as_deref(),
        Some(SENTINEL_LABEL_B),
        "two of the same resource keep their two distinct labels"
    );
}

/// A vehicle the side added that the template never carried has no label to inherit — and must
/// not steal one from a different resource.
#[test]
fn t373_merge_leaves_a_new_vehicle_unlabelled() {
    let stored = stored_template();
    let mut derived = derived_from_side();
    derived.vehicles = vec![veh(UAZ, None), veh(M151, None)];
    let merged = merge_faction_doc_from_side(&stored, derived).expect("ok");
    assert_eq!(merged.vehicles[0].label, None, "UAZ is new to the template");
    assert_eq!(merged.vehicles[1].label.as_deref(), Some(SENTINEL_LABEL_A));
}

/// The legitimate half of the button: roles really do follow the side, including a role the
/// side added, a role the side dropped, and a loadout the Arsenal cleared. These are all
/// expressible on a slot, so they are derived, not preserved.
#[test]
fn t373_roles_follow_the_side() {
    let stored = stored_template();
    let mut derived = derived_from_side();
    derived.roles = vec![role("Squad Leader", None), role("Medic", Some(loadout()))];
    let merged = merge_faction_doc_from_side(&stored, derived).expect("ok");
    let names: Vec<&str> = merged.roles.iter().map(|r| r.role.as_str()).collect();
    assert_eq!(
        names,
        vec!["Squad Leader", "Medic"],
        "the side is the truth"
    );
    assert!(
        merged.roles[0].loadout.is_none(),
        "a cleared loadout is a real edit, not a gap to backfill"
    );
    assert!(merged.roles[1].loadout.is_some(), "and a new one lands");
}

/// The button updates a template; it does not rename one. `Save as` is the rename.
#[test]
fn t373_name_comes_from_the_stored_doc() {
    let stored = stored_template();
    let merged = merge_faction_doc_from_side(&stored, derived_from_side()).expect("ok");
    assert_eq!(merged.name, "US Army 1980s");
    assert_eq!(merged.side, "BLUFOR", "side still comes from the side tab");
}

/// A side with no squads yields no roles and no vehicles. That body is schema-valid (no
/// `minItems`) and would empty the library faction, so it is refused before any write.
#[test]
fn t373_empty_side_is_refused_outright() {
    let stored = stored_template();
    let empty = FactionDoc {
        side: "BLUFOR".into(),
        name: "BLUFOR".into(),
        ..Default::default()
    };
    // `FactionDoc` has no `Debug` (dto.rs), so unwrap the Result by hand rather than
    // `expect_err`.
    let Err(err) = merge_faction_doc_from_side(&stored, empty) else {
        panic!("an empty side must be refused, never written");
    };
    assert_eq!(
        err,
        crate::error::Error::SideHasNoContent {
            stored_roles: 2,
            stored_vehicles: 2
        }
    );
    let msg = err.message_for_side("BLUFOR", &stored.name);
    assert!(
        msg.contains("BLUFOR") && msg.contains("US Army 1980s"),
        "{msg}"
    );
    assert!(msg.contains('2'), "names what would be lost: {msg}");
}

/// Refusal is scoped to a **no-content** write (T-348's precedent), not to any list shrinking to
/// zero: a vehicle-only side is a legitimate motor-pool template.
#[test]
fn t373_vehicle_only_side_is_allowed_but_warns() {
    let stored = stored_template();
    let mut derived = derived_from_side();
    derived.roles = Vec::new();
    let merged = merge_faction_doc_from_side(&stored, derived).expect("content-bearing");
    assert!(merged.roles.is_empty());
    assert_eq!(merged.emblem.as_deref(), Some(SENTINEL_EMBLEM));
    let warning = save_from_side_shrink_warning(&stored, &merged, "BLUFOR").expect("drops 2 roles");
    assert!(warning.contains("2 role(s)"), "{warning}");
}

#[test]
fn t373_shrink_warning_only_fires_when_content_is_removed() {
    let stored = stored_template();
    let same = merge_faction_doc_from_side(&stored, derived_from_side()).expect("ok");
    assert!(
        save_from_side_shrink_warning(&stored, &same, "BLUFOR").is_none(),
        "an equal-size update is one click"
    );

    let mut grown = derived_from_side();
    grown.roles.push(role("Medic", None));
    grown.vehicles.push(veh(UAZ, None));
    let grown = merge_faction_doc_from_side(&stored, grown).expect("ok");
    assert!(
        save_from_side_shrink_warning(&stored, &grown, "BLUFOR").is_none(),
        "adding rows is not destructive"
    );

    let mut shrunk = derived_from_side();
    shrunk.roles.truncate(1);
    let shrunk = merge_faction_doc_from_side(&stored, shrunk).expect("ok");
    let warning = save_from_side_shrink_warning(&stored, &shrunk, "BLUFOR").expect("drops 1 role");
    assert!(
        warning.contains("1 role(s)") && warning.contains("0 vehicle(s)"),
        "{warning}"
    );
}
