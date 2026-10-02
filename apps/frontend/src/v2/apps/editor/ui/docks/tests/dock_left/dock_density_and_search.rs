//! Dock density and search tests for the left editor dock.

/// T-637 — the left dock's density work: the search row that fills the void, and the five decoration
/// buttons that used to sit at the bottom of it.
///
/// **NOTE ON THE PIN IDIOM.** Every source needle below is checked against the PRODUCTION half only
/// (`split("#[cfg(test)]").next()`), so a needle written here cannot satisfy itself. The T-696 pins
/// in the module above once did `contains()` against the WHOLE file, needle and assertion together —
/// T-759 fixed that; they now read `class_r_scrub`'s scrubbed production half.
use super::{
    filter_outliner, find_layer_label, first_folder_label, matches_query, TAB_LABEL_LAYERS,
    TAB_LABEL_PAD_PX, TAB_LABEL_PLACES, UPPERCASE_LABEL_ADVANCE_PX,
};
use crate::v2::apps::editor::shell::layout::{tw_len_px, DOCK_L, DOCK_PX, STUB_PX};
use crate::v2::apps::editor::ui::outliner::outliner::{NodeKind, OutlinerNode};

/// The file's production half — everything above the first test module. A needle checked against
/// this cannot be satisfied by a test's own source.
fn production() -> &'static str {
    super::test_source::dock_left_source()
        .split("#[cfg(test)]")
        .next()
        .expect("the production half precedes the test modules")
}

fn node(id: &str, label: &str, kind: NodeKind, children: Vec<OutlinerNode>) -> OutlinerNode {
    OutlinerNode {
        id: id.to_string(),
        label: label.to_string(),
        kind,
        children,
        is_leader: false,
        hidden: false,
        locked: false,
        hidden_effective: false,
        locked_effective: false,
        tooltip: String::new(),
    }
}

/// Build a small but structurally real tree: two folders, one nested, slots under each.
fn tree() -> Vec<OutlinerNode> {
    vec![
        node(
            "l1",
            "Assault",
            NodeKind::Folder,
            vec![
                node("s1", "Rifleman", NodeKind::Slot, vec![]),
                node("s2", "Medic", NodeKind::Slot, vec![]),
                node(
                    "l2",
                    "Support",
                    NodeKind::Folder,
                    vec![node("s3", "Machinegunner", NodeKind::Slot, vec![])],
                ),
            ],
        ),
        node(
            "l3",
            "Recon",
            NodeKind::Folder,
            vec![node("s4", "Sniper", NodeKind::Slot, vec![])],
        ),
    ]
}

/// **A tree filter must not lie about containment.** The two rules together: an own-hit keeps its
/// whole subtree, and a descendant-hit keeps the ANCESTOR PATH so the match renders at the right
/// depth under the right parent. A flat "keep matching labels" filter would satisfy neither, and
/// in a tree whose entire job is showing what is inside what, that is a wrong answer, not a
/// terser one.
#[test]
fn the_layer_filter_keeps_subtrees_and_ancestor_paths() {
    let t = tree();

    // Empty / blank query ⇒ everything, unchanged. The filter is OFF by default; this is the
    // path that runs on every doc rebuild.
    assert_eq!(filter_outliner(&t, ""), t);
    assert_eq!(filter_outliner(&t, "   "), t);

    // OWN HIT keeps the whole subtree: searching for a folder means wanting its contents.
    let own = filter_outliner(&t, "assault");
    assert_eq!(own.len(), 1, "only the Assault branch survives");
    assert_eq!(own[0].id, "l1");
    assert_eq!(
        own[0].children.len(),
        3,
        "an own-hit folder keeps every child — pruning them would show an empty folder that is \
         not empty"
    );

    // DESCENDANT HIT keeps the path, pruned to it. `Machinegunner` is two levels down.
    let deep = filter_outliner(&t, "machinegun");
    assert_eq!(deep.len(), 1);
    assert_eq!(deep[0].id, "l1", "the grandparent is kept as structure");
    assert_eq!(
        deep[0].children.len(),
        1,
        "…but only the branch that leads to the hit"
    );
    assert_eq!(deep[0].children[0].id, "l2");
    assert_eq!(deep[0].children[0].children[0].id, "s3");

    // A miss is a miss — not a silently-unfiltered tree.
    assert!(filter_outliner(&t, "zzz").is_empty());

    // Case-insensitive substring, because it is the SAME predicate the Locations tab uses.
    assert_eq!(filter_outliner(&t, "SNIP").len(), 1);
    assert!(matches_query("Sniper", "SNIP"));

    // PERTURB the rule that costs the most if it is wrong: a filter that dropped
    // non-matching ancestors would return the hit at the ROOT, at the wrong depth, under no
    // parent. State that defect as a value and check the real result differs from it.
    let orphaned: Vec<&str> = vec!["s3"];
    let real: Vec<&str> = deep.iter().map(|n| n.id.as_str()).collect();
    assert_ne!(
        real, orphaned,
        "PERTURB: a hit must not surface as a root — the tree's job is containment"
    );
}

/// **The five decoration buttons are gone.** They were `disabled=true` unconditionally, their
/// tooltips literally said "(visual only)", and only one passed `active=true` — a permanent claim
/// to a Hierarchy/Layers/Assets/History/Settings tab set that did not exist and could not be
/// reached. `mt-auto` is what marooned them at the floor of a 900 px void.
///
/// Checked on the production half only, so this test's own mention of the string cannot keep it
/// green (the T-759 hollow-pin trap).
#[test]
fn no_decoration_survives_in_the_left_dock() {
    let src = production();
    // Needle assembled so this source cannot satisfy it.
    let decoration = format!("({} only)", "visual");
    assert!(
        !src.contains(&decoration),
        "T-637: a control whose tooltip admits it does nothing is furniture; the dock must not \
         ship any"
    );
    assert!(
        !src.contains("strip_btn"),
        "T-637: the decoration strip's builder must be gone, not merely unused"
    );
    assert!(
        !src.contains("mt-auto"),
        "T-637: `mt-auto` was the marooning — it turned the dock's unused height into a layout \
         feature instead of giving it to the tree"
    );
    // Every remaining `disabled=true` in the dock must be gone too: the dock has no permanently
    // dead controls left at all.
    assert!(
        !src.contains("disabled=true"),
        "T-637: no permanently-disabled control may remain in the left dock"
    );
}

/// **The height goes to the tree.** The dock is a column ([`crate::v2::apps::editor::shell::layout::DOCK_L`]); the
/// tree region claims the remainder with `flex-1` and can shrink inside it with `min-h-0`. Both
/// tokens are load-bearing: without `flex-1` the void comes straight back, and without `min-h-0`
/// a flex child refuses to shrink below its content, so a long tree pushes the panel instead of
/// scrolling inside it.
#[test]
fn the_tree_claims_the_dock_height_the_decoration_used_to_hold() {
    let src = production();
    assert!(
        src.contains("min-h-0 flex-1 overflow-y-auto"),
        "T-637: the layers tree region must claim the dock's remaining height and scroll inside it"
    );
    // …and the Layers tab has the search row Eden fills that width with.
    assert!(
        src.contains("dock-left-layers-filter"),
        "T-637: the Layers tab needs a driveable filter row, like the Locations tab already had"
    );
    // The tree renders the FILTERED signal, not the raw one — otherwise the box is decoration
    // itself, which is the exact defect this ticket deleted five of. Read as "the first argument
    // at the `virtual_tree` call site", so leading whitespace cannot make the check vacuous.
    let call = src
        .find("virtual_tree(")
        .expect("the dock must still render a tree");
    let first_arg = src[call + "virtual_tree(".len()..]
        .split(',')
        .next()
        .unwrap_or("")
        .trim();
    assert_eq!(
        first_arg, "layer_nodes",
        "T-637: the tree must be fed the FILTERED node set (got `{first_arg}`), or the filter \
         box is itself decoration"
    );
    assert!(
        src.contains("filter_outliner(ns, &q)"),
        "T-637: the filtered set must come from the shared tree filter"
    );
}

/// **THE HEADER FITS, AND THAT IS ARITHMETIC NOW TOO.** The peer of the right dock's
/// `t637_tab_strip_budget`. At the equalised 240 px this row holds the collapse chevron, two tab
/// labels and a trailing verb; before this ticket the labels alone overran it, and because the
/// tab group carries `min-w-0` the row squeezed rather than overflowed — the first label wrapped
/// and nothing anywhere reported it.
///
/// The label widths come from [`UPPERCASE_LABEL_ADVANCE_PX`], which is a MEASURED ceiling (see
/// its doc comment), so lengthening a label fails here instead of wrapping in a browser.
#[test]
fn the_header_row_fits_the_dock() {
    let pad = tw_len_px(DOCK_L, "p-").expect("the dock states its padding");
    // The header's own `px-1` gutter sits inside the dock's padding.
    let budget = DOCK_PX - 2.0 * pad - 2.0 * 4.0;
    let cell =
        |label: &str| label.chars().count() as f64 * UPPERCASE_LABEL_ADVANCE_PX + TAB_LABEL_PAD_PX;
    // chevron (STUB_PX — its hit box must match the collapsed stub, so it is not ours to shrink)
    // + gap + tab + gap + tab | gap | the trailing verb cell (`size-5`).
    let gap = 4.0;
    let verb = 20.0;
    let row = STUB_PX + gap + cell(TAB_LABEL_LAYERS) + gap + cell(TAB_LABEL_PLACES) + gap + verb;
    assert!(
        row <= budget,
        "T-637: the header wants {row} px of a {budget} px dock row. It will not overflow — the \
         tab group carries `min-w-0`, so it will SQUEEZE, wrap a label and grow a line, which is \
         the failure mode that reports nothing"
    );
    // The cells refuse to be squeezed, so a future overrun is a visible overflow the eye catches
    // rather than a silent reflow.
    let src = production();
    assert_eq!(
        src.matches("shrink-0 rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase")
            .count(),
        2,
        "T-637: both tab states must be `shrink-0` — a squeezable cell hides an overrun"
    );
}

/// The measured ceiling is a CEILING. If someone raises it to make a longer label fit, this
/// fails: the number has a provenance (a headless-Chrome measurement of the real classes) and
/// widening it silently would make the budget above meaningless.
#[test]
fn the_measured_label_advance_is_still_an_upper_bound() {
    // The two worst per-character advances actually observed, in the widest fallback font.
    for (label, measured_total) in [(TAB_LABEL_LAYERS, 45.75), (TAB_LABEL_PLACES, 72.88)] {
        let per_char = measured_total / label.chars().count() as f64;
        assert!(
            per_char <= UPPERCASE_LABEL_ADVANCE_PX,
            "T-637: `{label}` measured {per_char} px/char, above the {UPPERCASE_LABEL_ADVANCE_PX} \
             px ceiling the header budget is computed from"
        );
    }
    assert!(
        UPPERCASE_LABEL_ADVANCE_PX < 10.0,
        "T-637: a ceiling loose enough to admit anything is not a ceiling"
    );
}

/// T-803 (O-9) — **the drop-target resolvers name the layer `ensure_layer` actually files into.**
/// `find_layer_label` answers only for a live `Folder` id (a `Slot`/nested `Folder` id resolves
/// through the recursion; a stray or non-Folder id gives `None`, the stale-pointer case
/// `ensure_layer` clears before it falls back). `first_folder_label` is that fallback — the first
/// top-level layer, which is where a placement lands when nothing is active, and the reason the
/// operator saw "it doesn't place in the root": the root/Unfiled bucket is never the destination.
#[test]
fn the_drop_target_resolvers_name_the_real_destination() {
    let t = tree();

    // The active layer, by id, at any depth: top-level and nested both answer with their label.
    assert_eq!(find_layer_label(&t, "l1").as_deref(), Some("Assault"));
    assert_eq!(
        find_layer_label(&t, "l2").as_deref(),
        Some("Support"),
        "a nested folder is a valid drop target; the walk must reach it"
    );
    assert_eq!(find_layer_label(&t, "l3").as_deref(), Some("Recon"));

    // A SLOT id is not a layer, and a stray id is nobody: both are the `None` that sends
    // `ensure_layer` to its fallback. If `find_layer_label` answered for a slot, the strip would
    // name a destination that cannot receive a placement.
    assert_eq!(
        find_layer_label(&t, "s1"),
        None,
        "a slot id is not a drop target — only Folder kinds answer"
    );
    assert_eq!(find_layer_label(&t, "ghost"), None);

    // The fallback destination is the FIRST top-level layer — the same `rows.first()`
    // `ensure_layer` uses when nothing is active. Naming it is the fix for the "root" surprise.
    assert_eq!(first_folder_label(&t).as_deref(), Some("Assault"));

    // Empty doc ⇒ no layer to name; the strip says "a new layer" (what `ensure_layer` mints).
    assert_eq!(first_folder_label(&[]), None);

    // PERTURB the rule the operator's complaint turns on: were the fallback the ROOT/Unfiled
    // bucket instead of the first real layer, the strip would promise a destination placements
    // never reach. State that wrong answer and assert the real one differs from it.
    let unfiled = node("__unfiled__", "Unfiled", NodeKind::Unfiled, vec![]);
    let mut with_unfiled = vec![unfiled];
    with_unfiled.extend(tree());
    assert_eq!(
        first_folder_label(&with_unfiled).as_deref(),
        Some("Assault"),
        "PERTURB: the fallback must skip the virtual Unfiled root — it is not a doc layer and \
         receives no placement"
    );
}

/// T-803 (O-9) — **the persistent drop-target affordance ships in the dock.** The active layer's
/// only indication was a hover tooltip; this pins the on-screen statement (the `data-testid` hook
/// the scripted acceptance clicks for, the "Placing into:" copy, and that it reads BOTH
/// `active_layer` and the resolver so it names the real destination, not a static string).
/// Checked on the scrubbed production half, so this test's own mention cannot keep it green.
#[test]
fn the_drop_target_affordance_ships() {
    let src = production();
    assert!(
        src.contains("dock-left-drop-target"),
        "T-803: the drop-target statement needs a stable test hook for the scripted acceptance"
    );
    assert!(
        src.contains("Placing into:"),
        "T-803: the affordance must NAME the destination on screen, not only in a hover tooltip"
    );
    assert!(
        src.contains("find_layer_label"),
        "T-803: the strip must resolve the ACTIVE layer's label, or it cannot name the target"
    );
    assert!(
        src.contains("first_folder_label"),
        "T-803: the strip must fall back to `ensure_layer`'s first-layer destination, or it \
         would lie about where a placement lands when nothing is active"
    );
}
