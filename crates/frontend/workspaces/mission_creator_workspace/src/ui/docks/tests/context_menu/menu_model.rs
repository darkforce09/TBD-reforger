use super::*;

fn sel(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

// ── hit-target / selection-retarget rule (the load-bearing logic) ───────────

#[test]
fn empty_ground_when_nothing_hit() {
    let t = resolve_target(None, &sel(&["a", "b"]));
    assert_eq!(t.take, MenuTake::EmptyGround);
    assert!(t.target_ids.is_empty());
    assert_eq!(t.retarget_to, None);
}

#[test]
fn hit_inside_selection_targets_the_whole_selection_no_retarget() {
    // Right-click a member of a multi-select → the menu acts on the group, selection untouched.
    let t = resolve_target(Some("b"), &sel(&["a", "b", "c"]));
    assert_eq!(t.take, MenuTake::OnEntity);
    assert_eq!(t.target_ids, sel(&["a", "b", "c"]));
    assert_eq!(
        t.retarget_to, None,
        "an already-selected hit must not retarget"
    );
}

#[test]
fn hit_outside_selection_retargets_to_the_hit_entity() {
    // Right-click an unselected entity → retarget to it (replace selection), exactly like a
    // left-click on an unselected object.
    let t = resolve_target(Some("z"), &sel(&["a", "b"]));
    assert_eq!(t.take, MenuTake::OnEntity);
    assert_eq!(t.target_ids, sel(&["z"]));
    assert_eq!(t.retarget_to, Some("z".to_string()));
}

#[test]
fn hit_with_empty_selection_retargets() {
    // Nothing selected, right-click an entity → it becomes the target/selection.
    let t = resolve_target(Some("z"), &[]);
    assert_eq!(t.take, MenuTake::OnEntity);
    assert_eq!(t.target_ids, sel(&["z"]));
    assert_eq!(t.retarget_to, Some("z".to_string()));
}

// ── keyboard navigation (up/down/Enter) ─────────────────────────────────────

#[test]
fn selectable_indices_skip_separators_and_disabled_rows() {
    let e = MenuTake::EmptyGround.entries();
    let sel = selectable_indices(&e);
    // Only the enabled leaves: Go Here (idx 0). Everything else in this take is disabled or a
    // separator.
    for &i in &sel {
        assert!(e[i].item.is_some() && e[i].enabled);
    }
    assert!(sel.contains(&0), "Go Here (enabled) must be selectable");
}

#[test]
fn step_highlight_walks_only_enabled_rows_and_clamps() {
    let e = MenuTake::OnEntity.entries();
    let selectable = selectable_indices(&e);
    assert!(
        selectable.len() >= 3,
        "Go Here + Edit Loadout + Attributes at least"
    );
    // First Down lands on the first selectable row.
    let first = step_highlight(&e, None, 1).unwrap();
    assert_eq!(first, selectable[0]);
    // First Up (from nothing) lands on the last selectable row.
    let last = step_highlight(&e, None, -1).unwrap();
    assert_eq!(last, *selectable.last().unwrap());
    // Down from the last clamps (no wrap).
    assert_eq!(step_highlight(&e, Some(last), 1), Some(last));
    // Up from the first clamps.
    assert_eq!(step_highlight(&e, Some(first), -1), Some(first));
    // A landed highlight is always an enabled leaf.
    assert!(e[first].enabled && e[first].item.is_some());
    assert!(e[last].enabled && e[last].item.is_some());
}

#[test]
fn step_highlight_none_when_no_selectable_rows() {
    // A take with no enabled rows never yields a highlight (Enter would then no-op).
    let all_off = vec![
        MenuEntry::sep(),
        MenuEntry::off(ContextItem::Grid, "Grid", None),
    ];
    assert_eq!(step_highlight(&all_off, None, 1), None);
    assert_eq!(step_highlight(&all_off, None, -1), None);
}

/* ─────────────── T-651 — Place Comment: the enabled row and its world point ─────────────── */

/// `Place Comment` is LIVE and lives on the empty-ground take ONLY (Eden omits it on the entity
/// take, `batch01_context_menu.md:221`). Both halves matter: enabled-and-present is what T-651
/// ships, and absent-on-entity is what keeps a right-click on a unit from placing a note on top
/// of it.
#[test]
fn place_comment_is_enabled_and_empty_ground_only() {
    let empty = MenuTake::EmptyGround.entries();
    let row = empty
        .iter()
        .find(|r| r.item == Some(ContextItem::PlaceComment))
        .expect("Place Comment on the empty-ground take");
    assert!(row.enabled, "T-651 shipped the feature");
    assert_eq!(row.blocked, None, "an enabled row names no blocking ticket");
    assert!(!row.submenu, "it is a leaf action, not a parent");
    assert!(
        !MenuTake::OnEntity
            .entries()
            .iter()
            .any(|r| r.item == Some(ContextItem::PlaceComment)),
        "still omitted from the on-entity take"
    );
    // It is reachable by keyboard, which is the practical meaning of "enabled" for this menu.
    assert!(
        selectable_indices(&empty)
            .iter()
            .any(|&i| empty[i].item == Some(ContextItem::PlaceComment))
    );
}

/// **THE PLACE GESTURE, AS AN EVENT SEQUENCE** (not a source pin): right-click empty ground at a
/// known world point → the menu that opens targets no entity, carries THAT point, and offers a
/// live `Place Comment` row. Those four facts together are what make the dispatch land the
/// annotation on the ground the operator clicked.
///
/// The second half is the one that actually catches regressions: a SECOND right-click elsewhere
/// must replace the point. A menu that cached the first click's world position would place every
/// later comment at the first spot — a bug invisible on any single-event test.
#[test]
fn right_click_sequence_carries_each_click_own_world_point() {
    // 1. Right-click empty ground at world (100, 200) with a live selection.
    let selection = vec!["s1".to_string(), "s2".to_string()];
    let first = resolve_target(None, &selection).at_world(100.0, 200.0);
    assert_eq!(first.take, MenuTake::EmptyGround);
    assert!(
        first.target_ids.is_empty(),
        "the empty-ground take acts on a POINT, not on the selection"
    );
    assert_eq!(
        first.retarget_to, None,
        "an empty-ground right-click must not disturb the selection"
    );
    assert_eq!(first.world, Some((100.0, 200.0)));
    assert!(
        first
            .take
            .entries()
            .iter()
            .any(|r| r.item == Some(ContextItem::PlaceComment) && r.enabled),
        "the row the operator is about to click is live"
    );

    // 2. Dismiss, right-click again somewhere else: the NEW point wins.
    let second = resolve_target(None, &selection).at_world(7_000.5, -12.25);
    assert_eq!(
        second.world,
        Some((7_000.5, -12.25)),
        "each right-click carries its own point — no cached first click"
    );
    assert_ne!(first.world, second.world);

    // 3. A right-click ON an entity resolves to the other take, so `Place Comment` is not even
    //    offered — the world point riding along is inert there.
    let on_entity = resolve_target(Some("s1"), &selection).at_world(1.0, 2.0);
    assert_eq!(on_entity.take, MenuTake::OnEntity);
    assert!(
        !on_entity
            .take
            .entries()
            .iter()
            .any(|r| r.item == Some(ContextItem::PlaceComment))
    );
}

/// A host that supplies no world point (nothing to unproject against — no engine) leaves
/// `world` at `None`, and the dispatch's documented behaviour there is to do NOTHING rather than
/// guess a location. Pinned so `at_world` can never become implicitly-zero.
#[test]
fn a_target_without_a_world_point_stays_none() {
    assert_eq!(resolve_target(None, &[]).world, None);
    assert_eq!(resolve_target(Some("a"), &[]).world, None);
}

/* ═══════════════ T-672 — submenus, the two-act connect, the formation vocabulary ═══════════ */

/// Build the menu state a right-click on `hit` would produce, with `armed` as the connect that
/// was live at OPEN and `open` as the expanded parent. Mirrors what `open` + `toggle_submenu`
/// do on wasm, so the sequence tests below drive the same values the browser would.
fn state(hit: &str, armed: Option<(&str, &str)>, open: Option<ContextItem>) -> MenuState {
    MenuState {
        x: 10.0,
        y: 20.0,
        target: resolve_target(Some(hit), &[])
            .with_armed_connect(armed.map(|(k, f)| (k.to_string(), f.to_string()))),
        open_submenu: open,
    }
}

fn labels(entries: &[MenuEntry]) -> Vec<&'static str> {
    entries
        .iter()
        .filter(|r| r.item.is_some())
        .map(|r| r.label)
        .collect()
}

/// **`CONN-START-001` as an EVENT SEQUENCE, not a source pin.** The connect gesture is two
/// right-clicks, and this walks both of them through the same values the wasm host produces —
/// what the operator SEES at each step, and what the row they click resolves to.
///
/// The property that matters is that the Connect submenu shows exactly ONE of its two faces at
/// any moment. If it ever showed both, a click meant as "complete" could land on "re-arm" (or
/// vice versa) — an edge silently drawn between the wrong pair, which is precisely the class of
/// invisible defect this ticket's warning is about.
#[test]
fn the_two_act_connect_gesture_shows_one_face_at_a_time() {
    // ACT 1 — right-click the SOURCE. Nothing armed, so the submenu offers the three kinds.
    let s1 = state("s0", None, None);
    assert!(
        s1.entries()
            .iter()
            .any(|r| r.item == Some(ContextItem::Connect) && r.enabled && r.submenu),
        "Connect is a LIVE submenu parent after T-672"
    );
    assert_eq!(
        s1.entries(),
        MenuTake::OnEntity.entries(),
        "collapsed: no child rows are spliced until the parent is clicked"
    );

    let s1open = state("s0", None, Some(ContextItem::Connect));
    let l = labels(&s1open.entries());
    assert!(
        l.contains(&"Sync to") && l.contains(&"Group to") && l.contains(&"Set Trigger Owner"),
        "unarmed, the three KINDS are offered: {l:?}"
    );
    assert!(
        !l.contains(&"Complete Connection") && !l.contains(&"Cancel Connection"),
        "unarmed, there is nothing to complete: {l:?}"
    );
    // The children are spliced DIRECTLY under their parent, indented, not appended at the end.
    let entries = s1open.entries();
    let parent_at = entries
        .iter()
        .position(|r| r.item == Some(ContextItem::Connect))
        .expect("Connect row");
    assert_eq!(entries[parent_at + 1].label, "Sync to");
    assert!(entries[parent_at + 1].child, "child rows indent");
    assert!(!entries[parent_at].child, "the parent does not");
    // …and the child rows are keyboard-reachable, because they are in the same flat index space.
    assert!(
        selectable_indices(&entries).contains(&(parent_at + 1)),
        "a spliced child must be reachable by ArrowDown/Enter like any other enabled row"
    );

    // ACT 2 — the arm is live; right-click the TARGET. The SAME parent now offers the other face.
    let s2 = state("s1", Some(("sync", "s0")), Some(ContextItem::Connect));
    let l2 = labels(&s2.entries());
    assert!(
        l2.contains(&"Complete Connection") && l2.contains(&"Cancel Connection"),
        "armed, the completion pair is offered: {l2:?}"
    );
    assert!(
        !l2.contains(&"Sync to") && !l2.contains(&"Group to"),
        "armed, the kind rows are GONE — one face at a time: {l2:?}"
    );
    // The pending relation is legible from the row rather than remembered.
    let complete = s2
        .entries()
        .into_iter()
        .find(|r| r.item == Some(ContextItem::ConnectComplete))
        .expect("Complete row");
    assert_eq!(complete.note.as_deref(), Some("sync from s0"));
    // The target of act 2 is the entity right-clicked SECOND, which is what `dispatch` passes to
    // `complete_connect` — the whole point of the retarget rule holding across both acts.
    assert_eq!(s2.target.target_ids, vec!["s1".to_string()]);
    assert_eq!(s2.target.retarget_to.as_deref(), Some("s1"));
}

/// Expanding is not acting, and only the two shipped parents expand: a live `Connect` row means
/// "opens", not "does".
#[test]
fn submenu_parents_expand_they_do_not_act() {
    assert!(ContextItem::Connect.is_submenu_parent());
    assert!(ContextItem::Transform.is_submenu_parent());
    for leaf in [
        ContextItem::GoHere,
        ContextItem::Attributes,
        ContextItem::PlaceComment,
        ContextItem::ShowConnections,
        ContextItem::ConnectComplete,
    ] {
        assert!(!leaf.is_submenu_parent(), "{leaf:?} is a leaf");
        assert!(
            leaf.submenu_entries(None).is_empty(),
            "{leaf:?} has no children"
        );
    }
    // The still-disabled Eden parents (`Select` / `Edit` / `Grid` / `Log`) did NOT become
    // expandable: T-672 shipped two submenus, not submenus in general.
    for parked in [
        ContextItem::Select,
        ContextItem::Edit,
        ContextItem::Grid,
        ContextItem::Log,
    ] {
        assert!(!parked.is_submenu_parent(), "{parked:?} has no ticket yet");
    }
    // Only ONE submenu is open at a time — the state carries a single `Option`, so expanding
    // Transform cannot leave Connect's rows behind.
    let t = state("s0", None, Some(ContextItem::Transform));
    let l = labels(&t.entries());
    assert!(l.contains(&"Wedge"), "Transform expanded: {l:?}");
    assert!(!l.contains(&"Sync to"), "Connect stays collapsed: {l:?}");
}

/// **`CTX-FORMATION-001` names the SCHEMA's formations and no others.** Read out of
/// `mission.schema.json` at compile time rather than restated here, so the menu cannot offer a
/// formation the wire has no value for — the T-241 single-vocabulary rule, checked against the
/// authority instead of against a copy of it.
#[test]
fn the_formation_submenu_uses_the_schema_vocabulary_verbatim() {
    // Shared crate embed (wave-135 H2) — do not re-include_str the schema in this test.
    let schema: serde_json::Value =
        serde_json::from_str(mission_creator_state::zones::MISSION_SCHEMA).expect("schema parses");
    let want: Vec<String> = schema["$defs"]["group"]["properties"]["formation"]["enum"]
        .as_array()
        .expect("$defs/group.formation.enum")
        .iter()
        .map(|v| v.as_str().expect("string token").to_string())
        .collect();
    let got: Vec<String> = FormationKind::ALL
        .iter()
        .map(|f| (*f).token().to_string())
        .collect();
    assert_eq!(
        got, want,
        "menu formations must be the schema enum, in order"
    );

    // Every token reaches a row, and every row has a distinct human label.
    let rows = ContextItem::Transform.submenu_entries(None);
    assert_eq!(rows.len(), want.len());
    let mut seen: Vec<&str> = rows.iter().map(|r| r.label).collect();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), before, "two formations share a label");
}

/// The connect vocabulary agrees with itself in both directions, and rejects everything else —
/// the menu's own rows are built from.
#[test]
fn conn_kind_tokens_round_trip_and_reject_strangers() {
    for k in ConnKind::ALL {
        assert_eq!(ConnKind::parse(k.token()), Some(k));
    }
    for junk in ["", "Sync", "sync ", "attachedTo", "triggerowner"] {
        assert_eq!(ConnKind::parse(junk), None, "{junk:?} must not parse");
    }
}
