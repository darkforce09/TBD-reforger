//! Asset catalog search operators and safety tests.

use super::fixtures::*;
use super::*;

#[test]
fn parse_search_query_recognises_class_operator() {
    assert_eq!(
        parse_search_query("class:B_Soldier"),
        SearchQuery {
            field: SearchField::ClassName,
            pattern: SearchPattern::Plain("b_soldier".to_string()),
        },
        "class: → lowercased operand"
    );
    assert_eq!(
        parse_search_query("  CLASS: B_Soldier "),
        SearchQuery {
            field: SearchField::ClassName,
            pattern: SearchPattern::Plain("b_soldier".to_string()),
        },
        "operator is case-insensitive; leading/trailing space trimmed"
    );
    assert_eq!(
        parse_search_query("class:"),
        SearchQuery {
            field: SearchField::ClassName,
            pattern: SearchPattern::Pending,
        },
        "empty operand is recognised (and filters to nothing)"
    );
    assert_eq!(
        parse_search_query("rifleman"),
        SearchQuery {
            field: SearchField::Label,
            pattern: SearchPattern::Plain("rifleman".to_string()),
        }
    );
    assert_eq!(
        parse_search_query("classy"),
        SearchQuery {
            field: SearchField::Label,
            pattern: SearchPattern::Plain("classy".to_string()),
        },
        "`class` without the colon is a label"
    );
    assert_eq!(
        parse_search_query("first class:"),
        SearchQuery {
            field: SearchField::Label,
            pattern: SearchPattern::Plain("first class:".to_string()),
        },
        "the operator is a leading token, not a substring"
    );
}

#[test]
fn filter_catalog_class_prefix() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let rifleman_id =
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";

    let hit = filter_catalog(&tree, "class:{26A9756790131354}");
    assert_eq!(hit.len(), 1, "NATO kept via the one matching descendant");
    let leaves = &hit[0].children[0].children;
    assert_eq!(leaves.len(), 1, "only the prefix-matching leaf survives");
    assert_eq!(leaves[0].id, rifleman_id);
    assert!(
        leaves[0].payload.is_some(),
        "the survivor is the placeable leaf"
    );

    let all = filter_catalog(&tree, "class:{");
    assert_eq!(
        all[0].children[0].children.len(),
        8,
        "all classnames share the GUID-brace start"
    );

    let lower = filter_catalog(&tree, "class:{26a9756790131354}prefabs");
    assert_eq!(
        lower.len(),
        1,
        "lowercased operand matches the mixed-case id"
    );
    assert_eq!(lower[0].children[0].children[0].id, rifleman_id);

    assert!(
        filter_catalog(&tree, "class:{ZZZZ}").is_empty(),
        "a non-matching class prefix yields the empty tree"
    );
    assert!(
        filter_catalog(&tree, "class:Rifleman").is_empty(),
        "class: is prefix-only — a mid-classname token does not match"
    );

    assert!(
        filter_catalog(&tree, "class:").is_empty(),
        "class: with an empty operand matches nothing"
    );
    assert!(
        filter_catalog(&tree, "class:   ").is_empty(),
        "class: with whitespace-only operand also matches nothing"
    );

    assert_eq!(
        filter_catalog(&tree, "nato"),
        tree,
        "an un-prefixed query is still the T-055 label substring match"
    );
}

#[test]
fn class_empty_operand_has_its_own_empty_message() {
    assert_eq!(
        search_empty_message("class:", "assets"),
        "Type a class name after class:",
        "empty operand → guidance, not a miss"
    );
    assert_eq!(
        search_empty_message("class:   ", "vehicles"),
        "Type a class name after class:",
        "whitespace-only operand is still empty"
    );
    assert_eq!(
        search_empty_message("class:zzz", "objects"),
        "No objects match."
    );
    assert_eq!(
        search_empty_message("rifleman", "assets"),
        "No assets match."
    );
}

#[test]
fn class_tail_matches_a_bare_classname_on_a_real_guid_headed_id() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let rifleman_id =
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";
    assert_eq!(tree[0].children[0].children[0].id, rifleman_id);
    assert_eq!(
        classname_tail(rifleman_id),
        "Character_US_Rifleman",
        "the tail is the last path segment minus the extension"
    );

    let bare = filter_catalog(&tree, "class:Character_US_Rifleman");
    assert_eq!(bare.len(), 1, "a bare classname must not empty the tree");
    assert_eq!(bare[0].children[0].children.len(), 1);
    assert_eq!(bare[0].children[0].children[0].id, rifleman_id);

    let partial = filter_catalog(&tree, "class:character_us_ri");
    assert_eq!(
        partial[0].children[0].children[0].id, rifleman_id,
        "tail matching is a prefix, and case-insensitive"
    );

    let guid = filter_catalog(&tree, "class:{26A9756790131354}Prefabs");
    assert_eq!(guid[0].children[0].children[0].id, rifleman_id);

    assert!(
        filter_catalog(&tree, "class:Rifleman").is_empty(),
        "tail matching is prefix-only, not substring"
    );
    let globbed = filter_catalog(&tree, "class:*Rifleman");
    assert_eq!(
        globbed[0].children[0].children[0].id, rifleman_id,
        "a mid-classname token is reachable as a glob"
    );
}

#[test]
fn mod_operator_filters_by_the_addon_root() {
    let tree = build_vehicle_catalog_tree(&vehicle_items());
    assert_eq!(
        tree[0].label, "ArmaReforger",
        "guard: the root is the addon"
    );
    let all_leaves = tree[0].children[0].children.len();

    let hit = filter_catalog(&tree, "mod:ArmaReforger");
    assert_eq!(hit, tree, "an addon hit keeps the addon's whole subtree");

    assert_eq!(
        filter_catalog(&tree, "mod:arma"),
        tree,
        "`mod:` is a PREFIX and case-insensitive"
    );
    assert_eq!(
        filter_catalog(&tree, "mod ArmaReforger"),
        tree,
        "the space-separated spelling is the same operator"
    );
    assert!(
        filter_catalog(&tree, "mod:TBD_Framework").is_empty(),
        "an addon that is not in the tree empties it"
    );
    assert!(
        filter_catalog(&tree, "mod:Wheeled").is_empty(),
        "mod: matches the addon root only, not any folder"
    );
    assert!(
        !filter_catalog(&tree, "Wheeled").is_empty(),
        "guard: `Wheeled` is a real folder the label search finds"
    );
    assert!(all_leaves > 0, "guard: the fixture tree has leaves");
}

#[test]
fn glob_patterns_are_whole_string_over_the_selected_field() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let leaves = |t: &[CatalogNode]| -> Vec<String> {
        t.first().map_or_else(Vec::new, |root| {
            root.children[0]
                .children
                .iter()
                .map(|n| n.label.clone())
                .collect()
        })
    };

    assert_eq!(
        leaves(&filter_catalog(&tree, "*Medic")),
        ["US Medic"],
        "ends-with"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, "?S Medic")),
        ["US Medic"],
        "? is exactly one character"
    );
    assert!(
        filter_catalog(&tree, "?S Medicc").is_empty(),
        "the glob is whole-string: a trailing character it cannot consume is a miss"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, "*Anti-Tank")),
        ["US Light Anti-Tank"],
        "a glob spans spaces and punctuation"
    );

    let mg = filter_catalog(&tree, "class:*_MG");
    assert_eq!(leaves(&mg), ["US Machine Gunner"], "glob over the tail");
    let et = filter_catalog(&tree, "class:*Character_US_Medic.et");
    assert_eq!(
        leaves(&et),
        ["US Medic"],
        "glob over the full resource_name"
    );

    let vehicles = build_vehicle_catalog_tree(&vehicle_items());
    assert_eq!(
        filter_catalog(&vehicles, "mod:Arma*"),
        vehicles,
        "glob over the addon root"
    );
}

#[test]
fn glob_case_folding_is_symmetric_for_non_ascii() {
    let g = GlobPattern::parse("CAFÉ*");
    assert!(
        g.matches("café_x"),
        "CAFÉ* must match café_x when both sides use Unicode to_lowercase"
    );
    assert!(
        g.matches("CAFÉ_x"),
        "already-folded and mixed-case haystacks still match"
    );
    assert!(!g.matches("x_café"), "whole-string: leading junk is a miss");
    assert!(
        GlobPattern::parse("café*").matches("CAFÉ_x"),
        "fold is symmetric the other way too"
    );
}

#[test]
fn regex_patterns_search_the_selected_field() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let leaves = |t: &[CatalogNode]| -> Vec<String> {
        t.first().map_or_else(Vec::new, |root| {
            root.children[0]
                .children
                .iter()
                .map(|n| n.label.clone())
                .collect()
        })
    };

    assert_eq!(
        leaves(&filter_catalog(&tree, "/^us (medic|engineer)$/")),
        ["US Medic", "US Engineer"],
        "alternation + anchors over the label"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, "/anti-tank/")),
        ["US Light Anti-Tank"],
        "an un-anchored regex is a search, not a full match"
    );
    assert!(
        filter_catalog(&tree, "/^medic$/").is_empty(),
        "^ anchors: no label IS 'medic'"
    );

    assert_eq!(
        leaves(&filter_catalog(&tree, r"class:/^\{26a9756790131354\}/")),
        ["US Rifleman"],
        "an escaped GUID head"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, "class:/^{26A9756790131354}/")),
        ["US Rifleman"],
        "and an unescaped one — an open brace is a literal in this subset"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, r"class:/character_us_(mg|ar)\.et$/")),
        ["US Automatic Rifleman", "US Machine Gunner"],
        "alternation over the classname, in the tree's own order"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, r"class:/us_[a-z]{2}\.et$/")),
        Vec::<String>::new(),
        "a brace count is NOT a repetition in this subset — it is a literal, so this misses"
    );
    assert_eq!(
        leaves(&filter_catalog(&tree, r"class:/us_[a-l]+\.et$/")),
        ["US Grenadier"],
        "a character range + `+`: only `…_US_GL.et` is all a-l after the underscore"
    );
    let digit_guid = filter_catalog(&tree, r"class:/^.\d/");
    assert_eq!(digit_guid[0].children[0].children.len(), 7);
    assert!(
        !leaves(&digit_guid).contains(&"US Medic".to_string()),
        "the one GUID that starts with a letter is excluded"
    );

    let vehicles = build_vehicle_catalog_tree(&vehicle_items());
    assert_eq!(
        filter_catalog(&vehicles, "mod:/^arma(reforger|3)$/"),
        vehicles,
        "regex over the addon root"
    );
}

#[test]
fn a_broken_regex_says_so_instead_of_emptying_silently() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    for broken in ["/us(/", "/[a-/", "/*us/", "/us)/"] {
        assert_eq!(
            parse_search_query(broken).pattern,
            SearchPattern::Invalid,
            "{broken} must not parse"
        );
        assert!(filter_catalog(&tree, broken).is_empty());
        assert_eq!(
            search_empty_message(broken, "assets"),
            "That /…/ pattern could not be read — check the brackets and parentheses.",
            "a broken pattern reads as a syntax problem, not as a miss"
        );
    }
    assert_eq!(
        parse_search_query("/").pattern,
        SearchPattern::Plain("/".to_string())
    );
}

#[test]
fn every_operator_has_a_mid_type_empty_state() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    for (q, msg) in [
        ("class:", "Type a class name after class:"),
        ("mod:", "Type a mod name after mod:"),
        ("  MOD:  ", "Type a mod name after mod:"),
        ("//", "Type a pattern between the slashes."),
    ] {
        assert!(
            filter_catalog(&tree, q).is_empty(),
            "{q} is mid-type and must not show the whole tree"
        );
        assert_eq!(search_empty_message(q, "assets"), msg);
    }
    assert_eq!(filter_catalog(&tree, "   "), tree);
}

#[test]
fn a_catastrophic_regex_terminates_on_the_step_budget() {
    let long = "a".repeat(40);
    let items = vec![character_row(
        &format!("{{Z}}Prefabs/{long}b.et"),
        &long,
        "NATO/US_Army/Long",
    )];
    let tree = build_catalog_tree(&items, "BLUFOR");
    let _ = filter_catalog(&tree, "/^(a+)+$/");
    let _ = filter_catalog(&tree, "class:/(a|aa)+c/");
    assert_eq!(filter_catalog(&tree, "/^a+$/").len(), 1);
}

fn on_a_wasm_sized_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(body)
        .expect("spawn")
        .join()
        .expect("the matcher must return, not abort");
}

#[test]
fn deep_regex_input_refuses_instead_of_trapping_the_wasm_stack() {
    on_a_wasm_sized_stack(|| {
        for (label, pattern) in [
            (
                "nested parens",
                format!("{}a{}", "(".repeat(3000), ")".repeat(3000)),
            ),
            ("carets", "^".repeat(25_000)),
            ("dot-question", ".?".repeat(3000)),
        ] {
            assert!(
                Rx::parse(&pattern).is_none(),
                "{label} must be refused by RX_MAX_PATTERN"
            );
            assert_eq!(
                parse_search_pattern(&format!("/{pattern}/")),
                SearchPattern::Invalid,
                "{label} must reach the dock as Invalid"
            );
        }
        assert!(
            search_empty_message(&format!("/{}/", "^".repeat(25_000)), "assets")
                .contains("could not be read"),
            "an over-long pattern must be explained, not reported as an empty catalogue"
        );

        let src: Vec<char> = "(".repeat(512).chars().collect();
        let mut p = RxParser { src: &src, pos: 0 };
        assert!(p.alt().is_none(), "unbalanced parens must not compile");
        assert_eq!(p.pos, 512, "the parse must have entered all 512 levels");
        assert!(
            Rx::parse(&"(".repeat(512)).is_none(),
            "512 unbalanced parens must be refused cleanly, not trap"
        );
        assert_eq!(
            parse_search_pattern(&format!("/{}/", "(".repeat(512))),
            SearchPattern::Invalid,
            "the deepest pattern the length cap admits must reach the dock as Invalid"
        );
        assert!(
            Rx::parse(&"(".repeat(513)).is_none(),
            "one char past the cap must be refused for length"
        );
        assert_eq!(
            RX_MAX_PATTERN, 512,
            "re-measure the abort floor before changing this — see RX_MAX_PATTERN's doc"
        );

        let carets = Rx::parse(&"^".repeat(512)).expect("512 carets is within the length cap");
        assert_eq!(carets.search("abc"), (false, true));

        let evil = Rx::parse("(x+x+)+y").expect("the classic exponential pattern parses");
        assert_eq!(evil.search(&"x".repeat(3000)), (false, true));
        let tree = build_catalog_tree(
            &[character_row(
                "{Z}Prefabs/x.et",
                &"x".repeat(3000),
                "NATO/US_Army/X",
            )],
            "BLUFOR",
        );
        assert!(filter_catalog(&tree, "/(x+x+)+y/").is_empty());
    });
}

#[test]
fn honest_catalogue_patterns_stay_far_under_the_depth_cap() {
    on_a_wasm_sized_stack(|| {
        let hay = "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman_Long_Name_Variant.et";
        assert!(
            hay.chars().count() > 95,
            "subject must exceed the real worst case"
        );
        for pattern in [
            ".*rifleman.*",           // consumes the whole subject twice over
            "^\\{26a9.*\\.et$",       // anchored, GUID-headed, the documented idiom
            "(us|opfor|indfor)_army", // alternation across a group
            "[a-z_]+_rifleman",
        ] {
            let rx = Rx::parse(pattern).expect("honest pattern must compile");
            let (hit, capped) = rx.search(hay);
            assert!(hit, "{pattern} must still match");
            assert!(
                !capped,
                "{pattern} must not come anywhere near RX_MAX_DEPTH"
            );
        }
        let nested = Rx::parse(&format!("{}a{}", "(".repeat(200), ")".repeat(200)))
            .expect("401 chars is within the length cap");
        assert_eq!(nested.search("a"), (true, false));
    });
}

#[test]
fn multibyte_queries_do_not_panic_the_recogniser() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    for q in [
        "beauté",
        "abcdeß",
        "a日本",
        "clasé",
        "über-search",
        "日本語のクエリ",
    ] {
        let _ = filter_catalog(&tree, q);
        assert_eq!(
            parse_search_query(q).field,
            SearchField::Label,
            "{q} must stay Label"
        );
    }
    let _ = filter_catalog(&tree, "class:beauté");
}

#[test]
fn class_prefix_fires_where_label_cannot() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let token = "Character_US_Rifleman";

    assert!(
        filter_catalog(&tree, token).is_empty(),
        "guard: the classname token is absent from every label"
    );
    let classq =
        "class:{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman";
    let hit = filter_catalog(&tree, classq);
    assert_eq!(
        hit.len(),
        1,
        "class: fired: the classname-prefix leaf was selected"
    );
    assert_eq!(
        hit[0].children[0].children[0].label, "US Rifleman",
        "and it is the right leaf"
    );
}

#[test]
fn chip_side_then_class_search_compose() {
    let mut items = golden_items();
    items.push(character_row(
        "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
        "USSR Rifleman",
        "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
    ));

    let opfor = build_catalog_tree(&items, "OPFOR");
    assert!(
        filter_catalog(&opfor, "class:{26A9756790131354}").is_empty(),
        "chip filtered BLUFOR out before search; the BLUFOR class prefix cannot match"
    );
    let opfor_hit = filter_catalog(&opfor, "class:{DCB41B3746FDD1BE}");
    assert_eq!(
        opfor_hit.len(),
        1,
        "the OPFOR class prefix matches the USSR leaf"
    );

    let blufor = build_catalog_tree(&items, "BLUFOR");
    assert_eq!(
        filter_catalog(&blufor, "class:{26A9756790131354}").len(),
        1,
        "on the BLUFOR tree the BLUFOR class prefix hits — search runs on the chip-filtered tree"
    );
    assert!(
        filter_catalog(&opfor, "US Rifleman").is_empty(),
        "a label query for a BLUFOR role is empty under the OPFOR chip"
    );
}

#[test]
fn chip_side_predicate_per_side() {
    let us = character_row(
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
        "US Rifleman",
        "ArmaReforger/Characters/Factions/BLUFOR/US_Army/Rifleman",
    );
    let ussr = character_row(
        "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
        "USSR Rifleman",
        "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
    );
    let fia = character_row(
        "{84B40583F4D1B7A3}Prefabs/Characters/Factions/INDFOR/FIA/Character_FIA_Rifleman.et",
        "FIA Rifleman",
        "ArmaReforger/Characters/Factions/INDFOR/FIA/Rifleman",
    );
    assert!(character_matches_eden_side(&us, "BLUFOR"));
    assert!(!character_matches_eden_side(&us, "OPFOR"));
    assert!(!character_matches_eden_side(&us, "INDFOR"));
    assert!(character_matches_eden_side(&ussr, "OPFOR"));
    assert!(!character_matches_eden_side(&ussr, "BLUFOR"));
    assert!(character_matches_eden_side(&fia, "INDFOR"));
    assert!(!character_matches_eden_side(&fia, "BLUFOR"));
    assert!(!character_matches_eden_side(&us, "CIV"));
    assert!(!character_matches_eden_side(&us, ""));
}
