//! Document search tests for the left editor dock.

use super::{DocEntity, DocHit, DocKind, matches_query, search_document, selection_facets};

fn entity(id: &str, kind: DocKind, label: &str, faction: &str) -> DocEntity {
    DocEntity {
        id: id.into(),
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
        mission_creator_state::asset_catalog::search_empty_message(q, "entities in this mission")
    };
    assert!(msg("class:").contains("class:"), "guidance, not `no match`");
    assert!(msg("/[/").contains("could not be read"));
    assert!(msg("nosuchthing").contains("No entities in this mission match"));
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
                mixed.iter().any(|e| e.id == **id),
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
