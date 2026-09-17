use super::{resolve_target, ContextItem, MenuEntry, MenuState, ARRANGE};

/// The state a right-click on `ids[0]` produces while `ids` are all selected — the multi-select
/// case (`resolve_target` targets the WHOLE selection when the hit is inside it).
fn state(ids: &[&str], open: Option<ContextItem>) -> MenuState {
    let selection: Vec<String> = ids.iter().map(|s| (*s).to_string()).collect();
    MenuState {
        x: 12.0,
        y: 34.0,
        target: resolve_target(Some(ids[0]), &selection),
        open_submenu: open,
    }
}

fn labels(entries: &[MenuEntry]) -> Vec<&'static str> {
    entries
        .iter()
        .filter(|e| e.item.is_some())
        .map(|e| e.label)
        .collect()
}

/// THE DEFECT: two or more entities selected is exactly when Arrange applies, and the menu the
/// selection gesture opens never mentioned it.
#[test]
fn a_multi_selection_offers_arrange() {
    let rows = state(&["a", "b"], None).entries();
    assert!(
        labels(&rows).contains(&"Arrange"),
        "T-939.4: the right-click menu over a multi-selection offers no Arrange row — the \
         align / space / orient tools are reachable only from the top strip. Rows: {:?}",
        labels(&rows)
    );
}

/// **The acceptance line, and it is a line about ABSENCE.** One selected entity has nothing to
/// align against and no gaps to distribute, so the parent is not rendered at all — not rendered
/// dark, not rendered empty. A greyed row would still teach the operator that the tools exist
/// here, which is the one thing worth having; a dark row they cannot use at the moment they
/// look at it teaches them the opposite. The T-668 rule is no clickable no-op, and the honest
/// reading of it for a whole submenu of no-ops is to leave it out.
///
/// The empty-ground take is checked too: a right-click on bare terrain has no target at all, so
/// there is nothing for Arrange to act on however much is selected elsewhere.
#[test]
fn a_single_selection_hides_arrange_entirely() {
    for rows in [
        state(&["only"], None).entries(),
        // Even with the parent nominated as expanded, a single selection must not conjure it —
        // otherwise `open_submenu` would be a back door around the gate.
        state(&["only"], Some(ContextItem::Arrange)).entries(),
        MenuState {
            x: 0.0,
            y: 0.0,
            target: resolve_target(None, &["a".into(), "b".into()]),
            open_submenu: Some(ContextItem::Arrange),
        }
        .entries(),
    ] {
        let l = labels(&rows);
        assert!(
            !l.contains(&"Arrange"),
            "T-939.4: Arrange must be absent below the two-entity floor; rows: {l:?}"
        );
        assert!(
            !rows.iter().any(|e| matches!(
                e.item,
                Some(ContextItem::ArrangeRun(_)) | Some(ContextItem::Arrange)
            )),
            "T-939.4: no Arrange row of any kind may be spliced in; rows: {l:?}"
        );
    }
}

/// **The acceptance line "entries match the top-strip menu", proved rather than asserted.** The
/// submenu is not a curated subset chosen to look similar — it is a `map` over the same
/// `ARRANGE` array the menu bar builds its rows from, so equality here is equality of the list
/// with itself. What this test really defends is that nobody later re-types the rows locally:
/// the day someone does, the order or a single label drifts and this goes red.
#[test]
fn the_submenu_is_the_top_strips_own_list() {
    let rows = state(&["a", "b"], Some(ContextItem::Arrange)).entries();
    let children: Vec<&MenuEntry> = rows.iter().filter(|e| e.child).collect();
    assert_eq!(
        children.iter().map(|e| e.label).collect::<Vec<_>>(),
        ARRANGE.iter().map(|e| e.label).collect::<Vec<_>>(),
        "T-939.4: the Arrange submenu must render the top strip's list, in its order"
    );
    for (child, src) in children.iter().zip(ARRANGE.iter()) {
        assert_eq!(
            child.item,
            Some(ContextItem::ArrangeRun(src.kind)),
            "T-939.4: `{}` must dispatch on its own id, not on its label",
            src.label
        );
        assert_eq!(
            child.shortcut, src.chord,
            "T-939.4: `{}` must advertise the chord that actually runs it",
            src.label
        );
        assert!(
            child.enabled,
            "T-939.4: `{}` ships today — a dark row would be a lie",
            src.label
        );
    }
    // The submenu is reachable and not a dead parent: the row that opens it expands rather than
    // acts, exactly like Connect and Transform.
    assert!(ContextItem::Arrange.is_submenu_parent());
    assert!(!ARRANGE.is_empty());
}

/// The parent's placement is behavioural, not cosmetic: it must sit next to `Transform`, the
/// other row that rearranges what is already selected, so the two an author reaches between are
/// adjacent rather than separated by the Grid / Log stubs.
#[test]
fn arrange_sits_beside_transform() {
    let rows = state(&["a", "b"], None).entries();
    let l = labels(&rows);
    let t = l.iter().position(|s| *s == "Transform").expect("Transform");
    let a = l.iter().position(|s| *s == "Arrange").expect("Arrange");
    assert_eq!(
        a,
        t + 1,
        "T-939.4: Arrange must follow Transform; rows: {l:?}"
    );
}
