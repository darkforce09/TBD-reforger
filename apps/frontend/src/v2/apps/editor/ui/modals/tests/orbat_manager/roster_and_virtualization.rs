use super::*;
use crate::v2::core::api::dto::{FactionDoc, FactionRole, FactionVehicle, UserFaction};

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

#[test]
fn g1_dialog_class_near_fullscreen() {
    assert!(DIALOG_CLASS.contains("w-[min("));
    assert!(DIALOG_CLASS.contains("max-w-6xl"));
    assert!(!DIALOG_CLASS.contains("max-w-xl"));
}

#[test]
fn g2_set_leader_symbol_in_module_source() {
    // Wiring lives in stitch_row → orbat_set_leader; keep a compile-time reminder.
    let src = super::source::production_source();
    assert!(
        src.contains("orbat_set_leader"),
        "G2 Make SL must call set_leader path"
    );
    assert!(src.contains("set_leader") || src.contains("orbat_set_leader"));
}

/// H5 — Cancel path never allows apply.
#[test]
fn apply_cancel_noop() {
    assert!(!apply_confirm_allows(false));
    assert!(apply_confirm_allows(true));
    let src = super::source::production_source();
    assert!(
        src.contains("apply_confirm_allows(confirmed)"),
        "Apply must gate on confirm"
    );
    assert!(
        src.contains("orbat_apply_faction"),
        "Apply must call editor_ops path"
    );
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

/// H8 — Add Vehicle wiring present (not a disabled stub).
#[test]
fn orbat_add_vehicle_increases_vehicle_ids() {
    let src = super::source::production_source();
    assert!(
        src.contains("orbat_add_vehicle"),
        "Add Vehicle must call orbat_add_vehicle"
    );
    assert!(
        !src.contains("title=\"Add Vehicle (T-180.8)\"\n                                disabled"),
        "Add Vehicle must not stay disabled"
    );
    let ops = [
        crate::v2::core::test_support::editor_operations::ENTITY,
        crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
    ]
    .concat();
    assert!(
        ops.contains("pub fn orbat_add_vehicle"),
        "ops mutator must exist"
    );
    assert!(
        ops.contains("add_vehicle") && ops.contains("attach_vehicle"),
        "ops must call core add+attach"
    );
    let hist = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/document_host/history.rs"
    ));
    assert!(
        hist.contains("vehicles_bind"),
        "map presence: vehicles_bind on doc change"
    );
}

/// T-800 (F-21) — Add Vehicle on an EMPTY vehicle catalog must explain itself, not open a
/// pick-nothing select that changes nothing (the silent no-op the UX review flagged). The
/// empty-catalog branch renders BEFORE the normal picker (`vehicle_options.is_empty()` first),
/// carries a testid the acceptance script targets, and names the cause in operator words.
/// The explainer is inline in the row flow — same container class as the picker — so there is
/// no modal-stack z-order to consume (that only bites an overlay above the ORBAT dialog).
/// Needles fragment-assembled; the test module is not part of this haystack because these are
/// live view literals, not comment text.
#[test]
fn add_vehicle_empty_catalog_shows_explainer_not_silent_noop() {
    let src = super::source::production_source();
    let empty_guard = format!("picking_vehicle && {}", "vehicle_options.is_empty()");
    assert!(
        src.contains(&empty_guard),
        "T-800: the empty-catalog branch must gate on picking_vehicle AND an empty option list"
    );
    let testid = format!("data-testid=\"{}\"", "add-vehicle-empty-explainer");
    assert!(
        src.contains(&testid),
        "T-800: the explainer must carry the acceptance testid so a scripted click can assert it"
    );
    let cause = "No placeable vehicles in the active modpack";
    assert!(
        src.contains(cause),
        "T-800: the explainer must NAME why there is nothing to add, not fail silently"
    );
    // The normal picker still exists for the non-empty case — this is an added branch, not a
    // swap. `orbat_add_vehicle` (pinned by H8) remains reachable through the else-if arm.
    assert!(
        src.contains("} else if picking_vehicle {"),
        "T-800: the populated-catalog picker must remain as the else-if arm"
    );
}

/// I7 — OPEN ARSENAL opens Attributes on tab 3 (Arsenal), not Identity-only open_attributes.
#[test]
fn open_arsenal_selects_arsenal_tab() {
    let ops = crate::v2::core::test_support::editor_operations::CONTEXT;
    assert!(
        ops.contains("pub fn open_arsenal"),
        "open_arsenal must exist"
    );
    assert!(
        ops.contains("attrs_tab.set(3)"),
        "open_arsenal must select Arsenal tab index 3"
    );
    let mgr = super::source::production_source();
    assert!(
        mgr.contains("open_arsenal(id_ars"),
        "OPEN ARSENAL button must call open_arsenal"
    );
    // The Arsenal button path must not fall back to Identity-default open_attributes.
    let ars_idx = mgr
        .find("OPEN ARSENAL")
        .expect("OPEN ARSENAL label present");
    let window_start = ars_idx.saturating_sub(400);
    let window = &mgr[window_start..ars_idx];
    assert!(
        window.contains("open_arsenal"),
        "click handler near OPEN ARSENAL must call open_arsenal"
    );
    assert!(
        !window.contains("open_attributes"),
        "OPEN ARSENAL must not call open_attributes (Identity default)"
    );
    let attrs = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal/field_gates_and_labels.rs"
    ));
    assert!(
        attrs.contains(r#"["Transform", "Identity", "States", "Arsenal"]"#),
        "TABS[3] must be Arsenal"
    );
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
        SaveFromSideRefusal::NoContent {
            stored_roles: 2,
            stored_vehicles: 2
        }
    );
    let msg = err.message("BLUFOR", &stored.name);
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

/// The engine's hosted commands must keep naming the merge, and the button must keep calling
/// it — the wiring is what makes the rest of this file true.
#[test]
fn t373_save_button_merges_and_editor_ops_says_so() {
    let src = super::source::production_source();
    assert!(
        src.contains(
            "merge_faction_doc_from_side(\n                                        &stored.doc,"
        ) || src.contains("merge_faction_doc_from_side(&stored.doc"),
        "the Save button must PUT a merged body, never the raw derivation"
    );
    let ops = crate::v2::core::test_support::editor_operations::ENTITY;
    assert!(
        ops.contains("merge_faction_doc_from_side"),
        "faction_doc_from_side must name the merge callers have to use"
    );
    let context = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/host_state/editor_context/mod.rs"
    ));
    assert!(
        context.contains("#![cfg(target_arch = \"wasm32\")]"),
        "the editor context stays wasm-only, which is why the merge lives here where it is \
         testable"
    );
}

/// T-815 — squad rename focuses via NodeRef/on_load (wave200 F8).
#[test]
fn orbat_squad_rename_focuses_via_noderef_on_load() {
    // Scope to stitch_row live body so the ban needle cannot self-match this test's
    // string literal (include_str of the whole file always contains the assert text).
    use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};
    let code = live_source(&super::source::production_source());
    let body = only_body(&code, "fn stitch_row(");
    assert!(
        body.contains("NodeRef::<leptos::html::Input>::new()"),
        "the squad rename input must carry a NodeRef so it can be focused on mount"
    );
    assert!(
        body.contains("node_ref=rename_ref"),
        "the NodeRef must be attached via node_ref=rename_ref"
    );
    assert!(
        body.contains(".on_load(") && body.contains(".focus()") && body.contains(".select()"),
        "on_load must call focus() and select() on the mounted input"
    );
    assert!(
        body.contains("value=rename_draft.get_untracked()"),
        "rename input must seed via value= (uncontrolled after mount) so select-all sticks"
    );
    // T-726 concat pattern: fragments are not contiguous in this test source.
    let banned = ["prop:value=move || ", "rename_draft.get()"].concat();
    assert!(
        !body.contains(&banned),
        "reactive prop:value on squad rename clears on_load select-all — banned"
    );
    assert!(
        body.contains("data-testid=\"orbat-squad-rename\""),
        "rename input must expose data-testid=orbat-squad-rename for CDP probes"
    );
    assert!(
        body.contains("\"Escape\"") && body.contains("rename_squad.set(None)"),
        "Escape must abandon the rename session without relying on dialog close"
    );
}

/// T-726 — ORBAT Manager Esc must gate on modal_stack topmost (wave139 F3).
#[test]
fn orbat_manager_gates_escape_on_modal_stack() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let code = live_code(&super::source::production_source());
    let dialog = only_body(&code, "pub fn OrbatManagerDialog(");
    assert!(dialog.contains("install_orbat_dialog_lifecycle("));
    let body = only_body(&code, "fn install_orbat_dialog_lifecycle(");
    let reg = ["modal_stack", "::", "register("].concat();
    let top = ["modal_stack", "::", "is_topmost_open(modal_id)"].concat();
    let unreg = ["modal_stack", "::", "unregister(modal_id)"].concat();
    assert!(
        body.contains(&reg),
        "T-726: OrbatManagerDialog must register"
    );
    assert!(
        body.contains(&top),
        "T-726: OrbatManagerDialog must gate Escape on is_topmost_open"
    );
    assert!(
        body.contains(&unreg),
        "T-726: OrbatManagerDialog must unregister"
    );
}

/// F-17 (T-807) — the player-cap chip pluralizes its noun, so a single slot never reads
/// "1 slots". Source-pinned (literals kept) to the `OrbatManagerDialog` body: the naked
/// `" slots · server cap"` literal must be gone and the `== 1` conditional present.
#[test]
fn cap_label_pluralizes_the_slot_count() {
    use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};
    let code = live_source(&super::source::production_source());
    let body = only_body(&code, "pub fn OrbatManagerDialog(");
    // Concat so this test's own literals cannot self-match (T-726 idiom).
    let naked = [" slots", " \u{b7} server cap"].concat();
    assert!(
        !body.contains(&naked),
        "F-17: the hard-plural \" slots · server cap\" literal must be gone"
    );
    assert!(
        body.contains("if total_slots == 1"),
        "F-17: the cap label must pluralize with the `== 1` conditional"
    );
    assert!(
        body.contains("{total_slots} slot{}"),
        "F-17: the label must interpolate the pluralized suffix after `slot`"
    );
}
