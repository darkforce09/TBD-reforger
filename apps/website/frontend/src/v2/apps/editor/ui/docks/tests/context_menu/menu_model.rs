use super::*;

fn sel(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

// ── item model / transcription fidelity ─────────────────────────────────────

#[test]
fn empty_ground_take_matches_eden_batch() {
    // batch :119-128 — six interactive rows, two separators, in order.
    let e = MenuTake::EmptyGround.entries();
    let labels: Vec<&str> = e
        .iter()
        .filter(|r| r.item.is_some())
        .map(|r| r.label)
        .collect();
    assert_eq!(
        labels,
        vec![
            "Go Here",
            "Play from Here",
            "Select",
            "Edit",
            "Log",
            "Place Comment",
            // T-672 — the ONE row in this file that Eden does not have. It is the entry point
            // to the connection graph's SEE + CHECK panel, and it is on the empty-ground take
            // deliberately: auditing the graph is not an act on an entity, so requiring the
            // operator to find something to right-click first would put a hurdle in front of
            // the exact surface the ticket exists to guarantee.
            "Connections...",
        ]
    );
    // Two separators (after Play from Here, after Log).
    assert_eq!(e.iter().filter(|r| r.item.is_none()).count(), 2);
}

#[test]
fn on_entity_take_matches_eden_batch() {
    // batch :199-221 — the long take. `Place Comment` is absent; `Play as the Character`
    // replaces `Play from Here`.
    let e = MenuTake::OnEntity.entries();
    let labels: Vec<&str> = e
        .iter()
        .filter(|r| r.item.is_some())
        .map(|r| r.label)
        .collect();
    assert_eq!(
        labels,
        vec![
            "Connect",
            "Go Here",
            "Play as the Character",
            "Select",
            "Edit",
            "Transform",
            "Grid",
            "Log",
            // T-672 — the non-Eden audit row, on both takes (see the empty-ground pin).
            "Connections...",
            "Save Custom Composition...",
            "Find in Asset Browser...",
            "Find in Config Viewer...",
            "Edit Loadout...",
            "Reset Loadout",
            "Attributes...",
        ]
    );
    assert!(
        !labels.contains(&"Place Comment"),
        "Place Comment must be omitted from the on-entity take (batch :221)"
    );
    assert!(
        !labels.contains(&"Play from Here"),
        "Play from Here is replaced by Play as the Character on-entity (batch :204)"
    );
}

#[test]
fn enabled_rows_are_exactly_the_shipping_features() {
    // The only rows we light up are the ones whose feature exists today. Everything else is a
    // disabled Eden row (with or without an owning ticket). If a later ticket enables a row it
    // updates this list deliberately.
    let mut on: Vec<&str> = MenuTake::EmptyGround
        .entries()
        .into_iter()
        .chain(MenuTake::OnEntity.entries())
        .filter(|r| r.enabled)
        .map(|r| r.label)
        .collect();
    on.sort_unstable();
    on.dedup();
    // T-651 turned `Place Comment` on — the deliberate list update this test asks for. It is the
    // FIRST of T-664's six forward-contract rows to ship, and it shipped by matching on the
    // variant in `dispatch`, exactly as the id-enum contract intended.
    //
    // T-672 turned on `Connect` (`CONN-START-001`) and `Transform` (`CTX-FORMATION-001`) — the
    // second and third of the six — and added the non-Eden `Connections...` audit row. Both
    // parents are enabled in the sense this test means (clicking them does something) but they
    // are the first rows that EXPAND rather than act; `submenu_parents_expand_they_do_not_act`
    // below is what pins that distinction so "enabled" cannot quietly come to mean two things.
    assert_eq!(
        on,
        vec![
            "Attributes...",
            "Connect",
            "Connections...",
            "Edit Loadout...",
            "Go Here",
            "Place Comment",
            "Transform",
        ]
    );
}

#[test]
fn disabled_rows_that_have_an_owning_ticket_name_it() {
    // The six-ticket forward contract: each of these disabled rows must carry its blocking
    // ticket so a later agent can find its attachment point.
    // T-651 — `PlaceComment` LEFT this list: its feature shipped, so it is enabled and carries
    // no blocking ticket. T-672 — `Connect` and `Transform` left it for the same reason
    // (`CONN-START-001` / `CTX-FORMATION-001` shipped). The assertions below
    // (`unblocked_by() == None` for each shipped row) are what stop a stale ticket tag from
    // outliving the work — a disabled-looking row naming a closed ticket is how a feature gets
    // built twice.
    for (item, ticket) in [
        (ContextItem::PlaceComment, "PLACE-COMMENT-001 (T-651)"),
        (ContextItem::Connect, "CONN-START-001 (T-672)"),
        (ContextItem::Transform, "CTX-FORMATION-001 (T-672)"),
    ] {
        assert_eq!(
            item.unblocked_by(),
            None,
            "{ticket} shipped — {item:?} must not still name a blocking ticket"
        );
    }
    let want = [
        (ContextItem::SaveComposition, "COMP-SAVE-001"),
        (ContextItem::Select, "KEY-WP-001"),
    ];
    let all: Vec<MenuEntry> = MenuTake::EmptyGround
        .entries()
        .into_iter()
        .chain(MenuTake::OnEntity.entries())
        .collect();
    for (item, ticket) in want {
        let row = all
            .iter()
            .find(|r| r.item == Some(item))
            .unwrap_or_else(|| panic!("{item:?} must appear in a take"));
        assert!(
            !row.enabled,
            "{item:?} must be disabled (feature not shipped)"
        );
        assert_eq!(
            row.blocked,
            Some(ticket),
            "{item:?} must name its unblocking ticket {ticket}"
        );
        assert_eq!(item.unblocked_by(), Some(ticket));
    }
}

#[test]
fn item_id_is_an_enum_not_a_label() {
    // The forward contract in one assertion: the model keys on a typed variant, and a submenu
    // parent is a distinct id a later ticket matches on — not the string "Connect".
    assert_ne!(ContextItem::Connect, ContextItem::Transform);
    assert_eq!(ContextItem::Attributes.unblocked_by(), None); // enabled, no "disabled until"
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
    assert!(selectable_indices(&empty)
        .iter()
        .any(|&i| empty[i].item == Some(ContextItem::PlaceComment)));
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
    assert!(!on_entity
        .take
        .entries()
        .iter()
        .any(|r| r.item == Some(ContextItem::PlaceComment)));
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

/// Expanding is not acting, and only the two shipped parents expand. Without this the meaning of
/// `enabled` forks: `enabled_rows_are_exactly_the_shipping_features` counts `Connect` as live,
/// and this is what says live means "opens" and not "does".
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
        serde_json::from_str(crate::v2::apps::editor::ui::inspector::zones_panel::MISSION_SCHEMA)
            .expect("schema parses");
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
