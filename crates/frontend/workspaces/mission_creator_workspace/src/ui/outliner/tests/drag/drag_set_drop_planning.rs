//! Drag set drop planning tests for the outliner.

use super::*;

#[test]
fn test_plan_drop() {
    let drag = DragSet {
        anchor: "a".to_string(),
        ids: vec!["a".to_string(), "b".to_string()],
    };
    let descendants = |id: &str| -> Vec<String> {
        if id == "a" {
            vec!["c".to_string()]
        } else {
            vec![]
        }
    };
    // Drop into valid
    assert_eq!(
        plan_drop(&drag, "valid", descendants),
        Some(vec!["a".to_string(), "b".to_string()])
    );

    // Drop into self
    assert_eq!(plan_drop(&drag, "a", descendants), None);

    // Drop into child
    assert_eq!(plan_drop(&drag, "c", descendants), None);
}

/// T-946.86 (.83) — the WHOLE set survives the plan, not just the anchor.
///
/// This is the property the defect violated: the planner always returned every id, and the
/// drop path then threw all but `anchor` away by reading a different latch. Pinning
/// "len == ids.len()" here means a future edit that quietly narrows the plan to the anchor
/// fails rather than reproducing wave 255 silently.
#[test]
fn plan_drop_returns_every_dragged_id_not_just_the_anchor() {
    let ids: Vec<String> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let drag = DragSet {
        anchor: "a".to_string(),
        ids: ids.clone(),
    };
    let planned = plan_drop(&drag, "dest", |_| Vec::new()).expect("a clean drop is allowed");
    assert_eq!(
        planned, ids,
        "every dragged row must reach the drop, in order — moving only `anchor` is the \
         T-946.83 defect"
    );
    assert!(
        planned.len() > 1,
        "PERTURB: a plan that yields one id is the anchor-only drop this pin exists to catch"
    );
}

/// The refusal is per-SET, not per-anchor: a non-anchor member holding the destination in its
/// subtree sinks the whole drop. Otherwise a five-row drag could reparent a folder under its
/// own child as long as the ANCHOR was innocent.
#[test]
fn a_non_anchor_member_can_refuse_the_whole_drop() {
    let drag = DragSet {
        anchor: "a".to_string(),
        ids: vec!["a".to_string(), "parent".to_string()],
    };
    let descendants = |id: &str| -> Vec<String> {
        if id == "parent" {
            vec!["dest".to_string()]
        } else {
            vec![]
        }
    };
    assert_eq!(
        plan_drop(&drag, "dest", descendants),
        None,
        "a member whose subtree contains the destination must refuse the drop even when the \
         anchor is clean"
    );
}
