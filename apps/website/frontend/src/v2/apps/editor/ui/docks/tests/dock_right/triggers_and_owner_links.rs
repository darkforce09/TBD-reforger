use super::*;

/// T-079 (RIGHT-MODE-003) — the Triggers palette mode + tab exist and map to their own sub-mode.
/// The pure pins: tab-index → sub-mode reports Triggers for tab 5, and tab 5 is NOT one of the
/// pre-existing surfaces (so it did not silently reuse another tab's slot). Also that a
/// `PaletteKind::Trigger` variant was added (the palette-mode vocabulary the ticket asks to grow).
#[test]
fn triggers_tab_maps_to_its_own_submode() {
    assert_eq!(EdenSubmode::from_tab(5, false), EdenSubmode::Triggers);
    // Objects mode on the Triggers tab does not turn it into Objects (that split is the Factions
    // tab's alone) — tab index wins.
    assert_eq!(EdenSubmode::from_tab(5, true), EdenSubmode::Triggers);
    // The pre-existing surfaces keep their tabs.
    assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
    assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
    assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
    assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
    assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);

    // The PaletteKind vocabulary grew a Trigger variant with its own glyph + title (the mode
    // exists in the enum, not just the tab). Source-inspected because `PaletteKind` is a private
    // enum a native test cannot name without pulling the wasm-gated module graph. The needle is
    // assembled so this test's own text is not the thing that satisfies the check.
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ));
    let variant = ["PaletteKind", "::", "Trigger"].concat();
    assert!(
        src.contains(&variant),
        "the palette-mode vocabulary must carry a Trigger variant"
    );
}

/// T-079 (RIGHT-MODE-003) — the Triggers palette is a LIVE surface wired to the editor-ops
/// trigger seam, not a T-069-style stub. Source inspection (the `compositions_tab_is_wired_not
/// _stubbed` precedent): the panel is a wasm-only view a native test cannot mount.
///
/// **Every needle is assembled at run time** (this file searches itself, so a contiguous literal
/// would make an absence check unfailable and a presence check unpassable — the hard-won rule at
/// the top of this file). Each needle is split and re-joined.
#[test]
fn triggers_tab_is_wired_not_stubbed() {
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ));
    let call = |f: &str| format!("ops::{f}(");

    // The tab strip renders a Triggers tab at index 5.
    assert!(
        SRC.contains(&format!("tab_btn(5, {:?})", "Triggers")),
        "a Triggers tab must be in the tab strip"
    );
    // The panel dispatch routes tab 5 to the triggers panel.
    assert!(
        SRC.contains(&format!("5 => {}(", "triggers_panel")),
        "tab 5 must dispatch to the triggers panel"
    );
    // The panel reaches the trigger edit seam: name / activation / owner / rule / delete.
    for f in [
        "set_trigger_name",
        "set_trigger_activation",
        "set_trigger_owner",
        "set_trigger_rule",
        "delete_trigger",
    ] {
        assert!(
            SRC.contains(&call(f)),
            "the Triggers panel must call {f} (the trigger edit surface)"
        );
    }
    // The OWNER picker (CONN-TRG-OWNER-001) reads the placed-entity list AND the line reads its
    // resolved endpoints.
    assert!(
        SRC.contains(&call("placed_owner_options")),
        "the Owner picker must list placed entities via placed_owner_options"
    );
    assert!(
        SRC.contains(&call("owner_line_world")),
        "the owner-link line must resolve its endpoints via owner_line_world"
    );

    // The editor-ops seam actually exposes those functions AND the geometry reaches the core
    // trigger mutators (the claim the store round-trip rests on).
    let ops = crate::v2::core::test_support::editor_operations::ENTITY;
    for f in [
        "pub fn set_trigger_owner",
        "pub fn trigger_rows",
        "pub fn placed_owner_options",
        "pub fn owner_line_world",
    ] {
        assert!(ops.contains(f), "editor_ops must expose `{f}`");
    }
    assert!(
        ops.contains("core.add_circle_trigger(") || ops.contains("core.add_polygon_trigger("),
        "a trigger draw must reach the core trigger mutator"
    );
}

/// T-079 — the trigger AREA is a SECOND CONSUMER of the SHIPPED zone draw tool: the Triggers
/// panel arms the SAME `begin_zone_draw` / `begin_zone_reshape` calls the Zones panel does, only
/// with `DrawTarget::Trigger`. This proves BOTH halves of the ticket's "parameterize, do not
/// fork" constraint:
///   • the zone tool is UNTOUCHED FOR ZONES — the Zones panel still arms with `DrawTarget::Zone`;
///   • no forked trigger draw state machine was invented — there is no `begin_trigger_draw` /
///     `advance_trigger_draw` / `close_trigger_polygon`; the trigger path routes through the
///     `zone_draw`/`zone_polygon` functions with the target flag.
#[test]
fn trigger_draw_is_second_consumer_of_the_zone_tool() {
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ));
    let zones_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/zones_panel.rs"
    ));
    // T-934.7 — the ops module was split; the no-forked-draw absence pins scan every submodule.
    let ops = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout_commands.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/slot_loadouts.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/composition_library.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/data/store/operations/compositions.rs"
        )),
        crate::v2::core::test_support::editor_operations::CONTEXT,
        crate::v2::core::test_support::editor_operations::ENTITY,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/selection_transform.rs"
        )),
    ]
    .concat();

    // Assemble the target tokens so this test's own source cannot satisfy the checks by accident.
    let trigger_target = ["Draw", "Target", "::", "Trigger"].concat();
    let zone_target = ["Draw", "Target", "::", "Zone"].concat();
    let begin_draw = ["begin_", "zone_draw"].concat();
    let begin_reshape = ["begin_", "zone_reshape"].concat();

    // The Triggers panel arms the SHARED draw tool, targeting triggers.
    assert!(
        SRC.contains(&begin_draw) && SRC.contains(&trigger_target),
        "the Triggers panel must arm the shared zone-draw tool with the Trigger target"
    );
    assert!(
        SRC.contains(&begin_reshape),
        "trigger reshape must route through the shared zone-reshape, not a forked one"
    );
    // The Zones panel is UNTOUCHED for zones — it still targets the Zone collection.
    assert!(
        zones_src.contains(&begin_draw) && zones_src.contains(&zone_target),
        "the Zones panel must still arm the zone-draw tool with the Zone target (untouched)"
    );

    // No forked trigger draw state machine exists anywhere: the geometry accumulation is the ONE
    // shared `advance_zone_draw` / `close_zone_polygon`. A `begin_trigger_draw` /
    // `advance_trigger_draw` / `close_trigger_polygon` would be exactly the fork the ticket bans.
    for forked in [
        ["begin_", "trigger_draw"].concat(),
        ["advance_", "trigger_draw"].concat(),
        ["close_", "trigger_polygon"].concat(),
    ] {
        assert!(
            !ops.contains(&forked),
            "found a FORKED trigger draw fn `{forked}` — the draw flow must be parameterized by \
             DrawTarget, not forked (the second-consumer constraint)"
        );
    }
    // The single per-collection branch really is on the target: the commit calls the trigger
    // mutators under a `DrawTarget::Trigger` match arm.
    assert!(
        ops.contains(&trigger_target) && ops.contains("core.add_circle_trigger("),
        "the commit's Trigger branch must call the core trigger mutator"
    );
}

/// T-079 (CONN-TRG-OWNER-001) — the owner-link LINE renders through the selection-overlay idiom
/// (a `pointer-events-none` SVG projected by the pure, native-tested `project_owner_line`), keyed
/// off the SELECTED trigger, and it TOLERATES a dangling owner by drawing nothing. Source pins;
/// the projection math + dangling tolerance are proven behaviourally by `project_owner_line`'s
/// native test (below) and the store's `owner_edge_assigns_clears_and_tolerates_dangling`.
#[test]
fn owner_line_uses_the_selection_overlay_idiom() {
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ));
    // Every SRC needle assembled at run time — this test's own source is part of the haystack, so
    // a contiguous literal would make a presence check unpassable-by-code (satisfied by the test
    // itself). The overlay is the ruler idiom: a non-interactive SVG projected by the pure helper.
    let non_interactive = ["pointer-events-", "none"].concat();
    let project_fn = ["project_", "owner_line"].concat();
    let resolve_fn = ["owner_", "line_world"].concat();
    assert!(
        SRC.contains(&non_interactive) && SRC.contains(&project_fn),
        "the owner line must be a pointer-events-none SVG drawn via the pure projection helper"
    );
    // Its endpoints come from the resolver that returns None (→ no line) when the owner dangles.
    assert!(
        SRC.contains(&resolve_fn),
        "the line's endpoints must come from the resolver (None on a dangling owner)"
    );
    // T-727 keying trap: the trigger LIST must not use a `<For>` keyed on the (repeatable) name.
    // Like the Zones list, it is a full `.map(...).collect_view()` re-render off `doc_tick`, so
    // there is no `<For>` node to mis-key — row identity is the trigger id, never its name.
    let list_label = ["Authored ", "triggers"].concat();
    assert!(
        SRC.contains(&list_label),
        "the trigger list must render (the authored-triggers list)"
    );
    let name_key = ["key=", "|t| t.name"].concat();
    assert!(
        !SRC.contains(&name_key),
        "the trigger list must not be <For>-keyed on the repeatable name (T-727)"
    );
}

/// T-079 (CONN-TRG-OWNER-001) — the owner-link line's projection is PURE and native-tested: two
/// world endpoints through a projector give the screen `<line>` endpoints. Perturb / restore: a
/// projector that scales + offsets must move BOTH endpoints through it (a bug that projected only
/// one end, or dropped the offset, fails here). The dangling-owner "draw nothing" path is proven
/// in the store test; this proves the geometry the overlay draws when there IS a line.
#[test]
fn project_owner_line_maps_both_endpoints() {
    use crate::v2::apps::editor::ui::inspector::zones_panel::project_owner_line;
    // Trigger centre (10,20) → owner (110,220), through a scale-2 + offset projector.
    let l = project_owner_line((10.0, 20.0), (110.0, 220.0), |x, y| {
        (x * 2.0 + 5.0, y * 2.0 + 7.0)
    });
    assert!(
        (l.x1 - 25.0).abs() < 1e-9 && (l.y1 - 47.0).abs() < 1e-9,
        "endpoint A not projected"
    );
    assert!(
        (l.x2 - 225.0).abs() < 1e-9 && (l.y2 - 447.0).abs() < 1e-9,
        "endpoint B not projected"
    );
    // Identity projector → world coords pass through unchanged (the two ends are distinct).
    let id = project_owner_line((1.0, 2.0), (3.0, 4.0), |x, y| (x, y));
    assert_eq!((id.x1, id.y1, id.x2, id.y2), (1.0, 2.0, 3.0, 4.0));
}

/// T-079 — `DrawTarget` is the second-consumer parameter, and its two variants are distinct
/// (so a zone draw and a trigger draw can never collapse into one). A tiny pin, but it is the
/// hinge the whole "one shared draw tool" design turns on.
#[test]
fn draw_target_variants_are_distinct() {
    use crate::v2::apps::editor::ui::inspector::zones_panel::DrawTarget;
    assert_ne!(DrawTarget::Zone, DrawTarget::Trigger);
    assert_eq!(DrawTarget::Trigger.noun(), "trigger");
    assert_eq!(DrawTarget::Zone.noun(), "zone");
}
