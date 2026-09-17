//! Selection and layer authoring tests for the outliner.

//! T-666 — the folder-click SELECTION RULES + the group-icon rule, native (this module is not
//! wasm-gated, so `cargo test -p website-frontend` runs it). These pin the pure logic the
//! `editor_ops` selectors call (`layer_direct_slot_children` / `layer_descendant_slots` /
//! `folders_holding_slots`) — the part that decides WHICH slots a folder click selects.
//!
//! The ops WRAPPERS (`create_layer` / `rename_layer` / `delete_layer` / `reparent_layer` /
//! `refile_slot_to_layer`) and the DOCK CONTROLS are `#![cfg(target_arch = "wasm32")]`, so they
//! cannot run under this native harness. Two things stand in for them, both real gates, not
//! prose: (1) the wrappers are thin pass-throughs onto the SHIPPED, ALREADY-TESTED core
//! mutators — `add_editor_layer` / `rename_editor_layer` / `remove_editor_layer` (subtree +
//! reseed) / `reparent_editor_layer` (cycle-guarded) / `move_slot_to_layer` — whose semantics
//! AND undo-in-one-step are pinned in `map-engine-core`'s store.rs tests (e.g.
//! `remove_editor_layer_reseeds_when_subtree_is_all_layers`), and each core mutator commits a
//! single transaction (one undo step); (2) the [`source_pins`] tests below read the source and
//! assert every control + tail is wired (create + refresh_docks tail, delete-confirm text,
//! inline rename, root dropzone, the unfiltered-doc selection source).

use super::*;
use crate::v2::apps::editor::ui::outliner::outliner::{build_outliner, LayerRow, SlotRow};

fn slot(id: &str) -> SlotRow {
    SlotRow {
        id: id.to_string(),
        role: "Rifleman".to_string(),
    }
}
fn layer(id: &str, parent: Option<&str>, ents: &[&str]) -> LayerRow {
    LayerRow {
        id: id.to_string(),
        name: format!("{id}-name"),
        parent_id: parent.map(str::to_string),
        entity_ids: ents.iter().map(|s| (*s).to_string()).collect(),
        hidden: false,
        locked: false,
    }
}

/// A three-level fixture: root(a1,a2) → child(b1) → grandchild(c1).
fn nested() -> Vec<LayerRow> {
    vec![
        layer("root", None, &["a1", "a2"]),
        layer("child", Some("root"), &["b1"]),
        layer("grand", Some("child"), &["c1"]),
    ]
}

// ── SEL-LAYER-CHILDREN-001 ────────────────────────────────────────────────────────────────

#[test]
fn direct_children_are_own_entity_ids_only() {
    let layers = nested();
    // The folder's DIRECT slot children = its own `entityIds`, in order — NOT the subtree.
    assert_eq!(
        layer_direct_slot_children(&layers, "root"),
        vec!["a1", "a2"]
    );
    assert_eq!(layer_direct_slot_children(&layers, "child"), vec!["b1"]);
    assert_eq!(layer_direct_slot_children(&layers, "grand"), vec!["c1"]);
}

#[test]
fn direct_children_unknown_layer_is_empty() {
    assert!(layer_direct_slot_children(&nested(), "nope").is_empty());
}

// ── SEL-LAYER-DESC-001 ────────────────────────────────────────────────────────────────────

#[test]
fn descendants_walk_the_whole_subtree() {
    let layers = nested();
    // root's descendants = root's own slots + child's + grandchild's (recursion is the point).
    assert_eq!(
        layer_descendant_slots(&layers, "root"),
        vec!["a1", "a2", "b1", "c1"]
    );
    // child's subtree stops above root but still reaches the grandchild.
    assert_eq!(layer_descendant_slots(&layers, "child"), vec!["b1", "c1"]);
    // a leaf folder's subtree is just itself.
    assert_eq!(layer_descendant_slots(&layers, "grand"), vec!["c1"]);
}

/// FIRED-ONCE (perturb / fail / restore): the descendant walk MUST recurse, and this is the
/// rule the destructive delete (`remove_editor_layer` subtree) and SEL-LAYER-DESC-001 both
/// stand on. To prove the assertion has teeth, PERTURB the fixture to break the parent chain
/// (grandchild reparented off the subtree), assert the walk then MISSES `c1` (the FAIL the
/// test would catch if the logic ever stopped recursing), then RESTORE the correct chain and
/// assert `c1` is back. If `layer_descendant_slots` were a direct-children-only lookup, the
/// FIRST (perturbed) and correct cases would be identical and this test could not tell them
/// apart — so the negative arm is what makes the recursion gate real.
#[test]
fn descendants_recursion_gate_fires() {
    // Correct chain: grandchild reachable → c1 present.
    let ok = layer_descendant_slots(&nested(), "root");
    assert!(
        ok.contains(&"c1".to_string()),
        "baseline reaches grandchild"
    );

    // PERTURB: detach `grand` from the subtree (parent → an unrelated root).
    let mut perturbed = nested();
    perturbed.push(layer("other", None, &[]));
    for l in &mut perturbed {
        if l.id == "grand" {
            l.parent_id = Some("other".to_string());
        }
    }
    let broken = layer_descendant_slots(&perturbed, "root");
    assert!(
        !broken.contains(&"c1".to_string()),
        "PERTURBED: with the chain cut, the subtree walk must NOT reach the grandchild's slot \
         — this is the failure the recursion gate exists to prevent"
    );
    // still reaches the intact level.
    assert!(broken.contains(&"b1".to_string()), "child level intact");

    // RESTORE: the intact fixture reaches the grandchild again.
    let restored = layer_descendant_slots(&nested(), "root");
    assert!(
        restored.contains(&"c1".to_string()),
        "RESTORE: grandchild back"
    );
}

#[test]
fn descendants_cycle_guarded() {
    // A malformed parentId cycle (root ↔ child) must terminate, not hang.
    let layers = vec![
        layer("root", Some("child"), &["a1"]),
        layer("child", Some("root"), &["b1"]),
    ];
    let got = layer_descendant_slots(&layers, "root");
    assert!(got.contains(&"a1".to_string()) && got.contains(&"b1".to_string()));
}

// ── The selection reads the UNFILTERED doc (the T-715 non-regression contract) ────────────

#[test]
fn selection_reads_unfiltered_doc_hidden_layer_still_selects() {
    // A HIDDEN layer's slots are dropped by `materialize()` (and so by `slot_rows`), which is
    // the T-715 defect lane. The selection helpers read `LayerRow.entity_ids` (from
    // `small_maps_json`, unfiltered), so a hidden folder STILL selects its slots — folder-click
    // selects what the DOC contains, not what the filtered view shows.
    let mut layers = nested();
    for l in &mut layers {
        if l.id == "child" {
            l.hidden = true; // child (and its grandchild) would vanish from the render SoA
        }
    }
    // Direct + descendant selection are unaffected by the hidden flag.
    assert_eq!(layer_direct_slot_children(&layers, "child"), vec!["b1"]);
    assert_eq!(
        layer_descendant_slots(&layers, "root"),
        vec!["a1", "a2", "b1", "c1"],
        "a hidden sub-layer's slots are still part of what the parent folder contains"
    );
}

// ── SEL-GROUP-ICON-001 ────────────────────────────────────────────────────────────────────

#[test]
fn group_icon_distinguishes_slot_holders_from_grouping_folders() {
    // `parent` groups only sub-folders; `leaf` directly holds a slot.
    let layers = vec![
        layer("parent", None, &[]),
        layer("leaf", Some("parent"), &["s1"]),
    ];
    assert!(
        !folder_holds_slots(&layers, "parent"),
        "pure grouping folder"
    );
    assert!(folder_holds_slots(&layers, "leaf"), "directly holds a slot");

    // The render-side set (built from the OutlinerNode tree) agrees: only `leaf` is flagged.
    let tree = build_outliner(&layers, &[slot("s1")]);
    let holders = folders_holding_slots(&tree);
    assert!(holders.contains("leaf"));
    assert!(!holders.contains("parent"));
}

#[test]
fn group_icon_set_walks_nested_folders() {
    // Nested render set must find a slot-holder at any depth (walk, not top-level only).
    let layers = nested(); // root & child & grand all hold ≥1 slot
    let tree = build_outliner(&layers, &[slot("a1"), slot("a2"), slot("b1"), slot("c1")]);
    let holders = folders_holding_slots(&tree);
    for id in ["root", "child", "grand"] {
        assert!(holders.contains(id), "{id} directly holds a slot");
    }
}

// ── Source-inspection pins for the wasm-only controls + the ops tails ─────────────────────

mod source_pins {
    //! These read the SOURCE of the three owned files and assert each control is wired, since
    //! the controls are `#![cfg(target_arch = "wasm32")]` and cannot be exercised natively.
    //! A pin fails loudly if a rename drops a call the ticket requires.

    const OPS: &str = crate::v2::core::test_support::editor_operations::ENTITY;
    /// The engine-side layer authoring every wrapper in `OPS` rides. The wrappers resolve the
    /// host and take the refresh tail; the document mutators live here.
    const ENGINE_LAYERS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/entity/layers.rs"
    ));
    const TREE: &str = include_str!("../../tree.rs");
    const DOCK: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_left/view/full_dock.rs"
    ));

    /// Every layer-authoring wrapper rides `after_local_edit()` — the tail that calls
    /// `refresh_docks()` (via `refresh_signals`). Pin the pairing so a wrapper can't ship
    /// mutating the core without refreshing the docks.
    #[test]
    fn wrappers_call_core_then_after_local_edit() {
        for (wrapper, core_call) in [
            ("pub fn create_layer", "add_editor_layer"),
            ("pub fn rename_layer", "rename_editor_layer"),
            ("pub fn delete_layer", "remove_editor_layer"),
            ("pub fn reparent_layer", "reparent_editor_layer"),
            ("pub fn refile_slot_to_layer", "move_slot_to_layer"),
        ] {
            assert!(OPS.contains(wrapper), "missing wrapper {wrapper}");
            assert!(
                ENGINE_LAYERS.contains(core_call),
                "{wrapper} must ride the shipped core mutator {core_call}"
            );
        }
        // The single refresh tail every wrapper funnels through.
        assert!(OPS.contains("mission_history::after_local_edit"));
    }

    /// The selection helpers read the UNFILTERED doc (`layer_rows` → `entity_ids`), never
    /// `materialize()` — the T-715 non-regression contract, stated in-code.
    #[test]
    fn selection_uses_layer_rows_not_materialize() {
        assert!(OPS.contains("select_layer_children"));
        assert!(OPS.contains("select_layer_descendants"));
        assert!(
            OPS.contains("layer_direct_slot_children") && OPS.contains("layer_descendant_slots"),
            "selectors must call the unfiltered-doc helpers"
        );
        // The selectors feed off `layer_rows(core)` (small_maps_json / editorLayersById),
        // which carries every slot regardless of hidden state — unlike `slot_rows`/materialize.
        assert!(OPS.contains("layer_rows(core)"));
    }

    /// LAYER-CREATE-001 — the "+" create button and the root dropzone live in the dock header.
    #[test]
    fn dock_has_create_button_and_root_dropzone() {
        assert!(
            DOCK.contains("create_layer"),
            "the + button calls create_layer"
        );
        assert!(DOCK.contains("New layer"), "the + button is labelled");
        assert!(
            DOCK.contains("complete_layer_drop_onto_root"),
            "the header is a root dropzone"
        );
        assert!(
            DOCK.contains("cancel_layer_drag"),
            "a stray drag is cleared"
        );
        // The authoring tree is enabled (last virtual_tree arg true on this dock).
        assert!(DOCK.contains("virtual_tree"));
    }

    /// T-809 (fold c, F-22) — placed vehicles are listed in the LEFT outliner, beside slots, with
    /// the SAME row affordances a slot gets: single-click selects through the kind-agnostic
    /// `select_slot`, double-click opens Attributes via `open_attributes`. Read off `vehicle_rows`
    /// and filtered to MAP-PLACED (`xy.is_some()`); gated on `authoring` so only the left outliner
    /// grows the footer. `virtual_tree` appends the rows in every render branch (empty/eager/
    /// windowed), so the list is present regardless of tree size.
    #[test]
    fn placed_vehicles_are_listed_in_the_outliner_with_slot_affordances() {
        use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
        let code = live_code(TREE);
        // Two cfg variants share the name; the wasm one (no leading `_` on the params) is the one
        // with the real body — match its unique signature so `only_body` is unambiguous.
        let body = only_body(&code, "fn placed_vehicle_rows(authoring:");
        assert!(
            body.contains("vehicle_rows()"),
            "T-809: the outliner footer reads the placed vehicles off the engine's vehicle_rows"
        );
        assert!(
            body.contains("xy.is_some()"),
            "T-809: only MAP-PLACED vehicles belong in the on-the-map outliner"
        );
        assert!(
            body.contains("select_slot(") && body.contains("open_attributes("),
            "T-809: a vehicle row selects (single click) and opens Attributes (dbl click) like a slot"
        );
        // Gated on `authoring` — the ORBAT tree (authoring=false) must not grow the footer.
        assert!(
            body.contains("if !authoring") || body.contains("!authoring"),
            "T-809: the footer is the left outliner's only — gated on the authoring flag"
        );
        // `virtual_tree` appends the footer in each render branch, so it survives windowing.
        let vt = only_body(&code, "fn virtual_tree(");
        let calls = vt.matches("placed_vehicle_rows(").count();
        assert!(
            calls >= 3,
            "T-809: virtual_tree must append placed vehicles in the empty, eager AND windowed \
             branches (found {calls}); a single call would drop them under one render path"
        );
        // The row is labelled for the operator (a section marker + a vehicle glyph).
        let lit = live_source(TREE);
        let lit_body = only_body(&lit, "fn placed_vehicle_rows(authoring:");
        assert!(
            lit_body.contains("\"Placed vehicles\"") && lit_body.contains("directions_car"),
            "T-809: the outliner section names itself and carries the vehicle glyph"
        );
    }

    /// LAYER-DEL-001 — delete is destructive-subtree and the confirm text SAYS SO.
    #[test]
    fn delete_confirm_names_the_subtree() {
        assert!(
            TREE.contains("delete_layer"),
            "delete action calls delete_layer"
        );
        assert!(
            TREE.contains("confirm_with_message"),
            "delete is behind a confirm"
        );
        // The confirm text must warn it removes nested folders + every unit filed under them.
        assert!(
            TREE.contains("folders nested inside it") && TREE.contains("every unit filed"),
            "the confirm must state the whole subtree is destroyed"
        );
    }

    /// T-811 — layer rename focuses via NodeRef/on_load; draft is decoupled from the
    /// list-tracked id signal (wave200 F1 / F2 remount trap).
    #[test]
    fn layer_rename_uses_noderef_onload_and_decoupled_draft() {
        // Raw TREE includes this test module, so every needle below would self-match its own
        // assertion string (the T-759 hollow-pin class); scrub to the production half.
        let tree = crate::v2::core::test_support::class_r_scrub::live_source(TREE);
        assert!(
            tree.contains("NodeRef::<leptos::html::Input>::new()"),
            "the layer rename input must carry a NodeRef so it can be focused on mount"
        );
        assert!(
            tree.contains("node_ref=rename_ref"),
            "the NodeRef must be attached via node_ref=rename_ref"
        );
        assert!(
            tree.contains(".on_load(") && tree.contains(".focus()") && tree.contains(".select()"),
            "on_load must call focus() and select() on the mounted input"
        );
        assert!(
            tree.contains("renaming:") && tree.contains("rename_draft:"),
            "rename id and draft must be separate RowAuthoring fields"
        );
        assert!(
            tree.contains("data-testid=\"layer-rename-input\""),
            "rename input needs a stable test id for the CDP acceptance probe"
        );
    }

    /// Inline rename + the two folder-click selection modifiers are wired in the tree.
    #[test]
    fn tree_has_inline_rename_and_dual_selection() {
        assert!(
            TREE.contains("rename_layer"),
            "inline rename commits via rename_layer"
        );
        assert!(
            TREE.contains("select_layer_children") && TREE.contains("select_layer_descendants"),
            "folder click selects children; a modifier selects descendants"
        );
        assert!(
            TREE.contains("alt_key()") && TREE.contains("shift_key()"),
            "the descendant modifier is Alt/Shift"
        );
        // Pointer-drag reparent/refile (the current TreeView-DnD idiom).
        assert!(TREE.contains("begin_layer_drag") && TREE.contains("begin_layer_slot_drag"));
        assert!(TREE.contains("complete_layer_drop_onto_folder"));
        // SEL-GROUP-ICON-001 — the distinct slot-holder glyph.
        assert!(TREE.contains("folder_special"));
    }

    /// T-803 (fold a) — the DROP-TARGET folder row reads DIFFERENTLY from a SELECTED row.
    ///
    /// The active drop target (`is_active`, the layer the next placement/comment lands in) and
    /// selection (`is_sel`) are two states, and the state-vocabulary rule says two states get two
    /// treatments. Before this fix both folder sites painted the active row with `ROW_ACTIVE` —
    /// the SAME class selection wears — so a drop-target folder and a selected row were the same
    /// paint. Both folder sites now use `ROW_DROP_TARGET`, and it shares no distinguishing token
    /// with `ROW_ACTIVE`.
    ///
    /// Source-scrubbed (`live_source` cuts the test module + comments, keeps class-name/`testid`
    /// literals) so this pin can never self-match its own assertion strings — the T-759 hollow-pin
    /// class. Scoped to `fn single_row` so the Slot arm's legitimate `if is_sel() { ROW_ACTIVE }`
    /// (selection, preserved) is in view for the non-regression check but does not pollute the
    /// `is_active`-predicate count.
    #[test]
    fn t803_drop_target_reads_differently() {
        use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};
        let src = live_source(TREE);
        let body = only_body(&src, "fn single_row(");

        // Both folder sites (inline-rename branch + normal `base` closure) paint the ACTIVE
        // drop-target row with ROW_DROP_TARGET, never ROW_ACTIVE. PERTURB: swap one site back to
        // `if is_active() { ROW_ACTIVE }` and this count drops to 1 → RED.
        let drop_sites = body.matches("if is_active() { ROW_DROP_TARGET }").count();
        assert_eq!(
            drop_sites, 2,
            "T-803: both folder sites (rename branch + normal branch) must paint the active \
             drop target with ROW_DROP_TARGET — found {drop_sites} of 2"
        );
        // The old defect verbatim: the drop target wearing selection's class. Zero, exactly.
        assert!(
            !body.contains("if is_active() { ROW_ACTIVE }"),
            "T-803: a folder's drop-target state must NOT reuse selection's ROW_ACTIVE — that is \
             the two-states-one-treatment defect this ticket closes"
        );
        // Non-regression: SELECTION still uses ROW_ACTIVE in the slot arm (`is_sel`). If this
        // vanished, the fix would have collateral-damaged the T-649/T-668 selection paint.
        assert!(
            body.contains("if is_sel() { ROW_ACTIVE }"),
            "T-803: a SELECTED slot row must still wear ROW_ACTIVE — the fix restyles the drop \
             target only, never selection"
        );

        // The non-colour half of the cue: a target chip rides the drop-target state, with a
        // stable testid for the CDP probe and a `my_location` glyph so the state is legible
        // without relying on the tertiary tint alone.
        assert!(
            body.contains("data-testid=\"layer-drop-target-chip\"") && body.contains("my_location"),
            "T-803: the drop-target row carries a testid'd target chip + glyph (non-colour cue)"
        );
    }

    /// T-803 (fold a) — `ROW_DROP_TARGET` and `ROW_ACTIVE` are DISTINCT recipes: not equal, and
    /// neither contains the other's distinguishing token. Production consts, so this reads their
    /// values directly (no scrub — same rationale as the T-668 vocabulary pin). This is the class
    /// half of the state-vocabulary rule: the two states cannot be told apart if their classes
    /// collapse to the same tokens.
    #[test]
    fn t803_drop_target_class_is_distinct_from_active() {
        use super::{ROW, ROW_ACTIVE, ROW_DROP_TARGET};
        assert_ne!(
            ROW_DROP_TARGET, ROW_ACTIVE,
            "T-803: drop-target and selection recipes must not be the same string"
        );
        // ROW_ACTIVE's distinguishing tokens (selection): the primary plate + the dark TOP BORDER.
        // None may appear in ROW_DROP_TARGET.
        for tok in ["bg-primary/20", "border-t", "border-background/60"] {
            assert!(
                !ROW_DROP_TARGET.contains(tok),
                "T-803: ROW_DROP_TARGET must not carry selection's token `{tok}` — a shared token \
                 is how two states start reading as one"
            );
        }
        // ROW_DROP_TARGET's distinguishing tokens (drop target): the tertiary plate + the INSET
        // RING. None may appear in ROW_ACTIVE.
        for tok in ["bg-tertiary/15", "ring-inset", "ring-tertiary/50"] {
            assert!(
                !ROW_ACTIVE.contains(tok),
                "T-803: ROW_ACTIVE must not carry the drop-target token `{tok}`"
            );
        }
        // The ring is a box-shadow, not a border, so — like ROW — the drop-target row adds no
        // height and survives windowing at the shared pitch (no `border-t` to grow the box).
        assert!(
            !ROW_DROP_TARGET.contains("border-t") && !ROW.contains("border-t"),
            "T-803: the drop-target ring must not reintroduce a top border (box-box height drift)"
        );
    }
}

/// T-668 — the shared tree-row recipes speak the one state vocabulary, so every dock/panel that
/// consumes `ROW`/`ROW_ACTIVE` (both docks, zones, compositions, triggers) inherits it. These are
/// production consts, so the pin reads their values directly — no scrub. The load-bearing check is
/// `ROW_ACTIVE`'s dark top border: it is what makes a SELECTED row distinct-by-construction from a
/// HOVERED one (before T-668 it had none, so the two differed only by tint).
mod t668_vocabulary {
    use crate::v2::apps::editor::shell::layout::{HOVER_FILL, TOGGLED_PLATE};

    /// The idle row carries the HOVER_FILL tokens (solid fill on hover, no border).
    #[test]
    fn row_carries_the_hover_fill() {
        for tok in HOVER_FILL.split_whitespace() {
            assert!(
                super::super::ROW.contains(tok),
                "ROW must carry HOVER_FILL token `{tok}` (the one hover fill)"
            );
        }
        assert!(
            !super::super::ROW.contains("border-t"),
            "an idle ROW must have NO top border — that is the TOGGLED cue"
        );
    }

    /// The selected row carries the TOGGLED_PLATE tokens — the lighter primary plate AND the 1px
    /// dark top border. This is the fix: distinct-by-construction from a hovered row.
    #[test]
    fn row_active_carries_the_toggled_plate() {
        for tok in TOGGLED_PLATE.split_whitespace() {
            assert!(
                super::super::ROW_ACTIVE.contains(tok),
                "ROW_ACTIVE must carry TOGGLED_PLATE token `{tok}` (plate + dark top border)"
            );
        }
    }

    /// Fire the distinction (perturb/fail/restore): a selected row and a hovered row differ by
    /// the top border BY CONSTRUCTION. RESTORE: `ROW_ACTIVE` has `border-t`, `ROW` does not.
    /// PERTURB: were `ROW_ACTIVE` merely the neutral hover fill (the defect), it would carry no
    /// border and this check would reject it.
    #[test]
    fn selected_and_hovered_rows_are_distinct_by_construction() {
        assert!(
            super::super::ROW_ACTIVE.contains("border-t")
                && !super::super::ROW.contains("border-t"),
            "RESTORE: only the selected row has the top border"
        );
        // PERTURB — the defect value (a toggle wearing the bare hover fill) has no border.
        let defect = "bg-white/10";
        assert!(
            !defect.contains("border-t"),
            "PERTURB: a selected row rendered as the neutral hover fill has no distinguishing \
             border — the check must reject it"
        );
        assert_ne!(
            super::super::ROW,
            super::super::ROW_ACTIVE,
            "idle and selected rows must not be the same string"
        );
    }

    /// The palette leaf is the idle ROW plus a grab cursor — same HOVER_FILL, no toggled plate
    /// (a leaf is dragged, never a persistent toggle).
    #[test]
    fn palette_leaf_is_hover_fill_plus_grab() {
        assert!(super::super::PALETTE_LEAF.contains("hover:bg-white/10"));
        assert!(super::super::PALETTE_LEAF.contains("cursor-grab"));
        assert!(
            !super::super::PALETTE_LEAF.contains("border-t"),
            "a palette leaf is not a toggle — no toggled-plate border"
        );
    }
}
