//! Dock density and search tests for the left editor dock.

use super::{filter_outliner, find_layer_label, first_folder_label, matches_query};
use mission_creator_state::ids::OutlinerNodeId;
use mission_creator_state::outliner_model::{NodeKind, OutlinerNode};

fn node(id: &str, label: &str, kind: NodeKind, children: Vec<OutlinerNode>) -> OutlinerNode {
    OutlinerNode {
        id: OutlinerNodeId::new(id),
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
    assert_eq!(
        find_layer_label(&t, &"l1".into()).as_deref(),
        Some("Assault")
    );
    assert_eq!(
        find_layer_label(&t, &"l2".into()).as_deref(),
        Some("Support"),
        "a nested folder is a valid drop target; the walk must reach it"
    );
    assert_eq!(find_layer_label(&t, &"l3".into()).as_deref(), Some("Recon"));

    // A SLOT id is not a layer, and a stray id is nobody: both are the `None` that sends
    // `ensure_layer` to its fallback. If `find_layer_label` answered for a slot, the strip would
    // name a destination that cannot receive a placement.
    assert_eq!(
        find_layer_label(&t, &"s1".into()),
        None,
        "a slot id is not a drop target — only Folder kinds answer"
    );
    assert_eq!(find_layer_label(&t, &"ghost".into()), None);

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
