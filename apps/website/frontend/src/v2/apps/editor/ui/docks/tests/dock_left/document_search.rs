//! Document search tests for the left editor dock.

/// T-697 — document search and the selection filter.
///
/// **NOTE ON THE PIN IDIOM.** Every source needle below is checked against `class_r_scrub`'s scrubbed
/// PRODUCTION text (`live_code` / `live_source`), whose first pass cuts the test module outright. A
/// pin that can be satisfied by itself is not a pin. The T-696 module above used to do exactly that
/// — `SRC = include_str!(whole file)`, so its needles matched their own assertions and would have
/// survived the deletion of the code they pinned; T-759 pointed those pins at these same helpers.
use super::{
    hit_is_routable, matches_query, search_document, selection_facets, unselectable_reason,
    DocEntity, DocHit, DocKind, HIT_GAP_PX, HIT_ICON_PX, HIT_MIN_LABEL_PX, HIT_ROW_PAD_PX,
    LIST_SCROLLBAR_PX, MAX_DOC_HITS, UPPERCASE_LABEL_ADVANCE_PX,
};
use crate::v2::apps::editor::mission_editor::route_target;
use crate::v2::apps::editor::shell::layout::{tw_len_px, DOCK_L, DOCK_PX};
use crate::v2::apps::editor::ui::inspector::validation_panel::register_route_probe;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// The dock's own production text — comments, test modules and unreachable arms removed.
fn dock_code() -> String {
    live_code(super::test_source::dock_left_source())
}
/// The same, with string literals KEPT: for pins about copy and `data-testid`s that ship.
fn dock_source() -> String {
    live_source(super::test_source::dock_left_source())
}
/// The document index's production text (`editor_ops.rs` carries no test module of its own).
fn ops_code() -> String {
    live_code(crate::v2::core::test_support::editor_operations::ENTITY)
}

fn entity(id: &str, kind: DocKind, label: &str, faction: &str) -> DocEntity {
    DocEntity {
        id: id.to_string(),
        kind,
        label: label.to_string(),
        class_name: String::new(),
        faction: faction.to_string(),
        text: vec![("label", label.to_string()), ("id", id.to_string())],
    }
}

/// A realistic small mission: two BLUFOR slots (one with a callsign the role does not contain),
/// an OPFOR vehicle with a class name and no authored text, and a zone.
fn mission() -> Vec<DocEntity> {
    vec![
        DocEntity {
            id: "slot-1".into(),
            kind: DocKind::Slot,
            label: "Rifleman".into(),
            class_name: "{26A9756790131354}Prefabs/Characters/Character_US_Rifleman.et".into(),
            faction: "BLUFOR".into(),
            text: vec![
                ("role", "Rifleman".into()),
                ("callsign", "Alpha-1".into()),
                ("class", "Character_US_Rifleman".into()),
                ("id", "slot-1".into()),
            ],
        },
        DocEntity {
            id: "slot-2".into(),
            kind: DocKind::Slot,
            label: "Medic".into(),
            class_name: String::new(),
            faction: "BLUFOR".into(),
            text: vec![("role", "Medic".into()), ("id", "slot-2".into())],
        },
        DocEntity {
            id: "veh-1".into(),
            kind: DocKind::Vehicle,
            label: "UAZ469".into(),
            class_name: "{ABCD}Prefabs/Vehicles/UAZ469.et".into(),
            faction: "OPFOR".into(),
            text: vec![("class", "UAZ469".into()), ("id", "veh-1".into())],
        },
        DocEntity {
            id: "zone-1".into(),
            kind: DocKind::Zone,
            label: "Objective Alpha".into(),
            class_name: String::new(),
            faction: "OPFOR".into(),
            text: vec![("label", "Objective Alpha".into()), ("id", "zone-1".into())],
        },
    ]
}

fn ids(hits: &[DocHit]) -> Vec<&str> {
    hits.iter().map(|h| h.entity.id.as_str()).collect()
}

/// **THE DOCUMENT IS SEARCHED, AND THE CATALOGUE ALREADY WAS.** The ticket's whole existence:
/// `filter_catalog` had exactly one caller before this ticket (the right dock's palette), and a
/// placed vehicle / zone / trigger / marker / object was readable by its own panel and by
/// nothing else. These are the questions that had no answer.
#[test]
fn the_placed_document_is_searchable_by_text() {
    let m = mission();
    // A plain label search reaches a slot's role and a zone's label.
    assert_eq!(ids(&search_document(&m, "rifle")), ["slot-1"]);
    assert_eq!(ids(&search_document(&m, "objective")), ["zone-1"]);
    // …and, the point of the ticket, a VEHICLE — a kind no tree in this editor has ever held.
    assert_eq!(ids(&search_document(&m, "uaz")), ["veh-1"]);
}

/// **EVERY TEXT ATTRIBUTE, NOT JUST THE DISPLAY LABEL — and the hit says which one.** A slot's
/// callsign is not in its role, so a search that only saw the tree label would miss it. The
/// reported field is what stops a hit from being mysterious ("why did THAT match?").
#[test]
fn the_search_covers_every_text_attribute_and_names_the_one_that_matched() {
    let m = mission();
    let hits = search_document(&m, "alpha");
    // `Alpha-1` is slot-1's callsign; `Objective Alpha` is the zone's label.
    assert_eq!(ids(&hits), ["slot-1", "zone-1"]);
    assert_eq!(hits[0].field, "callsign");
    assert_eq!(hits[1].field, "label");
    // An id is a text attribute too: authors paste ids out of validation findings.
    assert_eq!(ids(&search_document(&m, "veh-1")), ["veh-1"]);
    assert_eq!(search_document(&m, "veh-1")[0].field, "id");
}

/// **ONE GRAMMAR — T-084's, not a second one.** The four pattern kinds and the three fields all
/// behave in the document exactly as they behave in the palette, because they ARE the palette's:
/// `class:` prefixes the resource name or its tail, `mod:` takes the faction group, `*`/`?` glob
/// whole-string per attribute, `/…/` is an unanchored regex.
/// T-776 — a plain faction hit must name `faction`, not the entity's first text attribute.
/// `BLUFOR` returns every BLUFOR entity via folder self-match (deliberate); lying that the
/// *name* matched is the honesty gap wave-119 NIT-4 named. Delete the faction branch in
/// [`search_document`] and this goes red.
#[test]
fn a_faction_only_hit_names_faction_not_the_first_text_attribute() {
    let e = entity("slot-1", DocKind::Slot, "Alpha 1-1", "BLUFOR");
    assert!(
        !e.text
            .iter()
            .any(|(_, v)| v.to_lowercase().contains("blufor")),
        "precondition: no text attribute contains the faction token"
    );
    let hits = search_document(std::slice::from_ref(&e), "BLUFOR");
    assert_eq!(ids(&hits), ["slot-1"]);
    assert_eq!(
        hits[0].field, "faction",
        "T-776: a faction-only hit must report field=faction, not `{}`",
        hits[0].field
    );
    // And a real name hit still names the attribute — faction must not steal the credit.
    let by_name = search_document(std::slice::from_ref(&e), "Alpha");
    assert_eq!(by_name[0].field, "label");
}

#[test]
fn the_query_grammar_is_t084s() {
    let m = mission();
    // `class:` — leaf-only, prefix, full resource name OR the classname tail (T-646/T-084).
    assert_eq!(
        ids(&search_document(&m, "class:Character_US_Ri")),
        ["slot-1"]
    );
    assert_eq!(ids(&search_document(&m, "class:{ABCD}Prefabs")), ["veh-1"]);
    // A bare label search must NOT behave like `class:` — `class:` stays a prefix.
    assert!(search_document(&m, "class:Rifleman").is_empty());
    // `mod:` — the depth-0 group, which in a document is the faction.
    assert_eq!(ids(&search_document(&m, "mod:OPFOR")), ["veh-1", "zone-1"]);
    // Glob, whole-string, per attribute.
    assert_eq!(ids(&search_document(&m, "Alpha-?")), ["slot-1"]);
    assert_eq!(ids(&search_document(&m, "Rifle*")), ["slot-1"]);
    // Regex, unanchored. NOT load-bearing (T-764's stack-depth defect lives on this arm) — it is
    // here because it comes free with the shared grammar, and nothing steers an author to it.
    assert_eq!(ids(&search_document(&m, "/^medic$/")), ["slot-2"]);
    // And the grammar is literally the palette's, not a copy of it.
    let src = dock_code();
    assert!(
        src.contains("asset_catalog::filter_catalog(")
            && src.contains("asset_catalog::search_empty_message("),
        "T-697: the document search must run T-084's matcher and T-084's empty states, not its own"
    );
    for reinvention in [
        "fn parse_search_pattern",
        "enum SearchPattern",
        "enum SearchField",
    ] {
        assert!(
            !src.contains(reinvention),
            "T-697: `{reinvention}` here would be a SECOND query language in one editor"
        );
    }
}

/// **ONE BOX, ONE MEANING.** The layers tree, the bookmarks list, the locations index and the
/// document search all go through [`query_hits`], so they cannot drift into four ideas of what
/// the filter box means. The plain behaviour T-696 wrote `matches_query` for is unchanged.
#[test]
fn one_predicate_serves_every_list_in_this_dock() {
    assert!(matches_query("Montignac", ""), "empty query matches all");
    assert!(matches_query("Montignac", "   "), "blank query matches all");
    assert!(matches_query("Montignac", "montignac"), "case-insensitive");
    assert!(matches_query("Montignac", "TIGN"), "substring, not prefix");
    assert!(!matches_query("Montignac", "levie"));
    // The grammar rides along for free.
    assert!(matches_query("Montignac", "Mont*"));
    assert!(
        !matches_query("Montignac", "class:Mont"),
        "no class name to match"
    );
    let dock = dock_code();
    assert!(
        only_body(&dock, "fn matches_query").contains("query_hits("),
        "T-697: `matches_query` must be the one matcher, or the tree and the search disagree"
    );
}

/// **A HALF-TYPED QUERY IS NOT A FAILED SEARCH**, and a blank one is not a request to list the
/// whole mission. T-084 already draws both lines; this reuses its sentences rather than writing
/// a fourth set.
#[test]
fn blank_and_half_typed_queries_find_nothing_and_say_which() {
    let m = mission();
    assert!(
        search_document(&m, "").is_empty(),
        "a blank box is not a query"
    );
    assert!(search_document(&m, "   ").is_empty());
    assert!(
        search_document(&m, "class:").is_empty(),
        "half-typed operator"
    );
    assert!(search_document(&m, "/[/").is_empty(), "unreadable regex");
    let msg = |q: &str| {
        crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(
            q,
            "entities in this mission",
        )
    };
    assert!(msg("class:").contains("class:"), "guidance, not `no match`");
    assert!(msg("/[/").contains("could not be read"));
    assert!(msg("nosuchthing").contains("No entities in this mission match"));
}

/// **RESULTS SELECT, OR THEY SAY THEY CANNOT — wog.md 14.6 / T-754 / wave 129 RV-1.**
///
/// A CORRESPONDENCE pin, and it is the correspondence that is checked, not a list: for every
/// `DocKind` it computes the CLICK's own answer (`mission_editor::route_target` over a document
/// with one id per kind) and requires [`super::hit_is_routable`] — which the view's
/// button-vs-inert branch is — to equal it. Both directions, with a non-vacuity assert that this
/// run saw at least one LIVE row and at least one INERT one.
///
/// **Why the old pin could not have caught RV-1.** It restated `DocKind::is_selectable`'s own
/// `Slot | Vehicle` constant back at it. When the router grew a `Zone` arm (T-754) and an
/// `Entity` arm (wave 129 F1) the constant went stale, zone and object hits kept rendering
/// `aria-disabled` over a click that would have selected, and the pin stayed green throughout —
/// because it was checking the kind list against itself. A guard that repeats its subject is not
/// a guard.
///
/// Perturbation RED: restore the hardcoded list — decide the row from
/// `matches!(kind, DocKind::Slot | DocKind::Vehicle)` instead of asking the registered probe.
#[test]
fn a_hit_row_is_a_live_affordance_iff_the_click_would_select() {
    // One id per kind, in the document the shipped resolver reads. Slot ids live in the SoA,
    // which is not in this root, so `is_slot` supplies them exactly as the router's caller does.
    // The last four maps are in the document and owned by NO selection surface — the router has
    // no arm for a briefing marker, a trigger, a comment or an editor layer.
    let root = serde_json::json!({
        "vehiclesById": { "veh-1": { "position": { "x": 10.0, "y": 20.0 } } },
        "entitiesById": { "obj-1": { "position": { "x": 30.0, "y": 40.0 } } },
        "zonesById": {
            "zone-1": { "shape": { "circle": { "x": 5.0, "z": 6.0, "r": 50.0 } } }
        },
        "factionsById": { "BLUFOR": { "briefing": { "markers": [{ "id": "mark-1" }] } } },
        "triggersById": { "trg-1": {} },
        "commentsById": { "cmt-1": {} },
        "editorLayersById": { "lay-1": {} }
    });
    fn is_slot(id: &str) -> bool {
        id == "slot-1"
    }
    // The probe is registered the way `mission_editor` registers it at mount: the SAME
    // resolution the click runs, asked as a question.
    let probe_root = root.clone();
    register_route_probe(std::rc::Rc::new(move |id: &str| {
        route_target(&probe_root, id, &is_slot).is_some()
    }));

    let mut live: Vec<DocKind> = Vec::new();
    let mut inert: Vec<DocKind> = Vec::new();
    for (id, kind) in [
        ("slot-1", DocKind::Slot),
        ("veh-1", DocKind::Vehicle),
        ("obj-1", DocKind::Object),
        ("zone-1", DocKind::Zone),
        ("mark-1", DocKind::Marker),
        ("trg-1", DocKind::Trigger),
        ("cmt-1", DocKind::Comment),
        ("lay-1", DocKind::Layer),
    ] {
        let hit = DocHit {
            entity: entity(id, kind, "Row", "BLUFOR"),
            field: "label",
        };
        // The ORACLE — what the click would actually do — computed independently of the view.
        let would_select = route_target(&root, id, &is_slot).is_some();
        assert_eq!(
            hit_is_routable(&hit),
            would_select,
            "RV-1: the {} row's affordance must EQUAL what a click on it would do (the router \
             says {would_select}). Painting inert over a live click is the same lie as painting \
             live over a dead one",
            kind.noun()
        );
        if would_select {
            live.push(kind);
        } else {
            inert.push(kind);
        }
    }

    // NOT VACUOUS: this pin is worth nothing unless it saw the affordance both ON and OFF.
    assert!(
        !live.is_empty() && !inert.is_empty(),
        "the correspondence must be exercised in both directions (saw live {live:?} / inert \
         {inert:?})"
    );
    // What the router reaches TODAY — reported by the resolver, not asserted into the view. The
    // day an arm is added or removed this line moves and the affordance moves with it, together.
    assert_eq!(
        live,
        vec![
            DocKind::Slot,
            DocKind::Vehicle,
            DocKind::Object,
            DocKind::Zone
        ],
        "T-655 + T-754 (zones) + wave-129 F1 (placed objects) are the shipped router's arms"
    );
    assert_eq!(
        inert,
        vec![
            DocKind::Marker,
            DocKind::Trigger,
            DocKind::Comment,
            DocKind::Layer
        ],
        "T-754: the router has no arm for these, so their rows must stay INERT — rendering one \
         as clickable is the defect this programme has filed three times"
    );
    for kind in &inert {
        let why = unselectable_reason(*kind);
        assert!(
            why.contains(kind.noun()),
            "an inert row must say WHY, naming the kind it is about"
        );
        assert!(
            !why.contains("slots and vehicles"),
            "RV-1: the reason may not assert a router limit the router does not have. It read \
             `resolves slots and vehicles only` for the whole of the zone and entity widenings"
        );
    }

    // THE OTHER AXIS — same document, different mount state (F6/F7). The resolver refuses while
    // the owning panel is unmounted (or before the editor mounts, or on the host build), and the
    // row must follow the RESOLVER, not the document.
    register_route_probe(std::rc::Rc::new(|_: &str| false));
    let zone = DocHit {
        entity: entity("zone-1", DocKind::Zone, "Objective Alpha", "OPFOR"),
        field: "label",
    };
    assert!(
        !hit_is_routable(&zone),
        "F6/F7: the resolver refuses this subject, so the row is inert — a fallback to \
         `route_target` here IS the dead click"
    );
    assert!(
        route_target(&root, "zone-1", &is_slot).is_some(),
        "the document resolves `zone-1` in BOTH phases; only the probe's answer moved, which is \
         exactly why the affordance may not be decided from the document"
    );

    // SOURCE SIDE — one decision, and the click is on the other end of it.
    let code = dock_code();
    assert!(
        code.contains("validation_panel::route_select_by_subject_id("),
        "T-655/T-697: a hit must select through the ONE registered router"
    );
    assert!(
        !code.contains("entity_selection::select_slot("),
        "T-697: a second click-to-select path is how the two drift apart"
    );
    assert!(
        only_body(&code, &format!("fn hit{}", "_is_routable"))
            .contains(&format!("subject_id{}", "_routes")),
        "RV-1: clickability must be the REGISTERED probe's answer — the one the click runs"
    );
    // NEGATIVES, over the whole of this file's LIVE half (the test module, which legitimately
    // calls the router to state the FACT the affordance is checked against, is cut first).
    assert_eq!(
        code.matches(&format!("is{}", "_selectable(")).count(),
        0,
        "RV-1: the hardcoded kind list is gone. It is a second copy of the router's reach, and \
         a copy is a thing that can go stale — this one did, silently, for two widenings"
    );
    assert_eq!(
        code.matches(&format!("route{}", "_target(")).count(),
        0,
        "RV-1: no live code in this view may resolve the router itself — that is a second \
         availability decision, and the click's is the one that counts"
    );
    // The inert branch exists and is not a disabled-looking button.
    let src = dock_source();
    assert!(
        src.contains("aria-disabled") && src.contains("unselectable_reason(kind)"),
        "T-754: an unselectable hit must render as inert text carrying its reason"
    );
    assert!(
        src.contains("dock-left-search-hit") && src.contains("dock-left-search-hit-inert"),
        "T-697: both row shapes must be driveable from a gate"
    );
}

/// **THE SELECTION FILTER ONLY OFFERS NARROWINGS THAT NARROW.** A chip that would keep the whole
/// selection selected is the T-754 mistake in a second costume, so a homogeneous selection yields
/// no chips at all rather than a row of no-ops.
#[test]
fn the_selection_filter_offers_only_proper_subsets() {
    let homogeneous = vec![
        entity("a", DocKind::Slot, "One", "BLUFOR"),
        entity("b", DocKind::Slot, "Two", "BLUFOR"),
        entity("c", DocKind::Slot, "Three", "BLUFOR"),
    ];
    assert!(
        selection_facets(&homogeneous).is_empty(),
        "nothing to narrow by ⇒ no chips, not chips that do nothing"
    );
    assert!(
        selection_facets(&homogeneous[..1]).is_empty(),
        "one row is not a selection to filter"
    );
    assert!(selection_facets(&[]).is_empty());

    let mixed = vec![
        entity("a", DocKind::Slot, "One", "BLUFOR"),
        entity("b", DocKind::Slot, "Two", "OPFOR"),
        entity("c", DocKind::Vehicle, "Truck", "OPFOR"),
    ];
    let facets = selection_facets(&mixed);
    let total = mixed.len();
    for f in &facets {
        assert!(
            !f.ids.is_empty() && f.ids.len() < total,
            "{f:?} narrows nothing"
        );
        for id in &f.ids {
            assert!(
                mixed.iter().any(|e| &e.id == id),
                "a chip must not invent an id"
            );
        }
    }
    // BOTH axes the ticket names, and the counts are real.
    let by = |axis: &str, label: &str| {
        facets
            .iter()
            .find(|f| f.axis == axis && f.label == label)
            .unwrap_or_else(|| panic!("missing {axis} chip {label}"))
    };
    assert_eq!(by("Type", "slot").ids, ["a", "b"]);
    assert_eq!(by("Type", "vehicle").ids, ["c"]);
    assert_eq!(by("Faction", "BLUFOR").ids, ["a"]);
    assert_eq!(by("Faction", "OPFOR").ids, ["b", "c"]);
}

/// Rows with no faction get their OWN chip rather than being dropped — "the ones that belong to
/// nobody" is a real narrowing, and dropping them would make the chip counts fail to sum.
#[test]
fn the_faction_axis_keeps_the_unfactioned() {
    let rows = vec![
        entity("a", DocKind::Comment, "Note", ""),
        entity("b", DocKind::Slot, "One", "BLUFOR"),
    ];
    let facets = selection_facets(&rows);
    let none = facets
        .iter()
        .find(|f| f.axis == "Faction" && f.label == "no faction")
        .expect("the unfactioned must be reachable");
    assert_eq!(none.ids, ["a"]);
    let sum: usize = facets
        .iter()
        .filter(|f| f.axis == "Faction")
        .map(|f| f.ids.len())
        .sum();
    assert_eq!(
        sum,
        rows.len(),
        "the faction chips must partition the selection"
    );
}

/// **NARROWING A SELECTION IS NOT A DOCUMENT EDIT** (T-642's line). It goes through
/// `set_slot_selection` — the selection-only tail a folder click takes — and must never reach the
/// history, or every filter chip would cost the author a Ctrl+Z.
#[test]
fn narrowing_the_selection_is_not_undoable() {
    let ops = ops_code();
    let body = only_body(&ops, "pub fn set_selection_ids");
    assert!(
        body.contains("set_slot_selection(ids)"),
        "T-697: the narrow must reuse the shipped selection-only tail"
    );
    for banned in [
        "after_local_edit",
        "remove_slots",
        "add_slot",
        "mission_history::",
    ] {
        assert!(
            !body.contains(banned),
            "T-642/T-697: narrowing a selection must not be a document edit, found {banned}"
        );
    }
    let dock = dock_code();
    let apply = only_body(&dock, "pub fn apply_selection");
    assert!(
        apply.contains("entity_selection::set_selection_ids("),
        "T-697: the chip must apply through the one seam"
    );
}

/// **EVERY PLACED COLLECTION IS INDEXED, OR THE SEARCH LIES.** Eight collections an author can
/// place into; a ninth arriving without a case in `document_entities` would be silently
/// unfindable, which is the failure the ticket is about.
#[test]
fn the_index_covers_every_placeable_collection() {
    let ops = ops_code();
    let domain = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/document_index.rs"
    )));
    let body = only_body(&domain, "pub fn document_entities");
    for kind in [
        "DocKind::Slot",
        "DocKind::Vehicle",
        "DocKind::Object",
        "DocKind::Marker",
        "DocKind::Zone",
        "DocKind::Trigger",
        "DocKind::Comment",
        "DocKind::Layer",
    ] {
        assert!(
            body.contains(kind),
            "T-697: `{kind}` is not indexed — it cannot be found"
        );
    }
    // Read-only: the index must not open a transaction on the way past.
    for banned in ["after_local_edit", "core.add_", "core.set_", "core.remove_"] {
        assert!(
            !body.contains(banned),
            "T-697: the document index is a READ, found {banned}"
        );
    }
    // The selection projection is DERIVED from the index, so the two cannot disagree about an
    // entity's kind or faction. The adapter routes to the one projection; the projection reads
    // the index.
    assert!(
        only_body(&ops, "pub fn selection_entities").contains("selection_entities(core, &sel)"),
        "T-697: the selection filter must go through the one projection"
    );
    let projection = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/entity/selection_index.rs"
    )));
    assert!(
        only_body(&projection, "pub fn selection_entities").contains("document_entities(core)"),
        "T-697: the selection filter must read the same rows the search does"
    );
}

/// **THE TWO NEW ROWS FIT 240 px, AND THAT IS ARITHMETIC — the T-637 rule.** The dock is width
/// budgeted and this is the third ticket in it this run; eyeballing is what produced the silent
/// squeeze T-637 had to go and measure. The header is NOT touched (there is no third tab — it
/// does not fit; see the section header), so what is added up here is the two body rows.
#[test]
fn the_search_rows_fit_the_dock() {
    let pad = tw_len_px(DOCK_L, "p-").expect("the dock states its padding");
    // A row spans the dock's inner width, less whatever the scrolling list's scrollbar claims.
    let budget = DOCK_PX - 2.0 * pad - LIST_SCROLLBAR_PX;

    // ── the hit row: [px-1] icon │gap│ label(flex, truncates) │gap│ badge ───────────────────
    // The badge is the widest kind noun; the measured UPPERCASE ceiling is a safe bound for a
    // `lowercase` cell, which is narrower per character in every font in the stack.
    let widest_noun = [
        DocKind::Slot,
        DocKind::Vehicle,
        DocKind::Object,
        DocKind::Marker,
        DocKind::Zone,
        DocKind::Trigger,
        DocKind::Comment,
        DocKind::Layer,
    ]
    .into_iter()
    .map(|k| k.noun().chars().count())
    .max()
    .expect("eight kinds");
    let badge = widest_noun as f64 * UPPERCASE_LABEL_ADVANCE_PX;
    let furniture = HIT_ROW_PAD_PX + HIT_ICON_PX + 2.0 * HIT_GAP_PX + badge;
    let label = budget - furniture;
    assert!(
        label >= HIT_MIN_LABEL_PX,
        "T-697: the hit row's furniture wants {furniture} px of a {budget} px row, leaving \
         {label} px for the name — under the {HIT_MIN_LABEL_PX} px floor a result stops being \
         readable and the list becomes badges beside ellipses"
    );

    // ── the facet chips: they WRAP, so only a single chip has to fit on its own ─────────────
    let chip = |label: &str| {
        // `px-1.5` (12 px) + the text, worst case a three-digit count.
        12.0 + (label.chars().count() + " (999)".len()) as f64 * UPPERCASE_LABEL_ADVANCE_PX
    };
    let widest_chip = chip("vehicle");
    assert!(
        widest_chip <= budget - HIT_ROW_PAD_PX,
        "T-697: a `{}` chip wants {widest_chip} px of a {} px row",
        "vehicle",
        budget - HIT_ROW_PAD_PX
    );
    assert!(
        dock_source().contains("flex flex-wrap gap-1"),
        "T-697: the chips must WRAP — a selection straddling many factions must grow a line, \
         not overrun the column or squeeze its neighbours"
    );

    // The DOM is bounded too: a 2,000-hit query renders 200 rows and says so.
    assert!(
        MAX_DOC_HITS <= 400,
        "a 240 px column cannot usefully mount more"
    );
    assert!(
        dock_source().contains("Found {total} — showing {shown}"),
        "T-697: a truncated list must report the FULL count, or the number is a lie"
    );
}

/// The tree keeps its own filter and its own input (T-637), fed the FILTERED node set. The
/// document search is ADDED beside it, not swapped for it — a search that emptied the tree it
/// sits above would have deleted a shipped feature to add one.
#[test]
fn the_document_search_does_not_replace_the_layer_filter() {
    let src = dock_source();
    assert!(
        src.contains("filter_outliner(ns, &q)") && src.contains("virtual_tree("),
        "T-637: the layers tree and its filter must survive this ticket"
    );
    assert!(
        src.contains("dock-left-layers-filter") && src.contains("dock-left-search-results"),
        "T-697: one box, two surfaces — both must be driveable"
    );
}
