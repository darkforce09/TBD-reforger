use super::fixtures::*;
use super::*;

// ── T-646 (RIGHT-SEARCH-002) — the `class:` recogniser ────────────────────────────────────────

/// The recogniser reads a leading `class:` and hands back a lowercased/trimmed operand; anything
/// else is a plain label query. The operator is a LEADING token only — a query that merely
/// contains it later stays a label match (Eden's grammar), and `class` without the colon is not
/// the operator.
///
/// T-084 reshaped [`SearchQuery`] from a flat enum into `{field, pattern}` (the operator and the
/// pattern are now independent axes), so these assertions read through the new pair. Every
/// BEHAVIOUR T-646 pinned is asserted unchanged.
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
    // Not the operator: a bare word, or `class:` appearing past the start.
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

/// `class:<prefix>` matches a LEAF by its classname (`id` = `resource_name`), prefix,
/// case-insensitively — HIT (one leaf), MISS (empty tree), CASE (lower/upper agree), and
/// EMPTY-OPERAND (`class:` alone ⇒ nothing). The un-prefixed label path is unchanged — proven by
/// re-running a label query and getting the historical result.
#[test]
fn filter_catalog_class_prefix() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let rifleman_id =
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";

    // HIT — a GUID prefix that only the Rifleman leaf carries. The NATO / US_Army folders survive
    // by descent; every non-matching sibling leaf is pruned.
    let hit = filter_catalog(&tree, "class:{26A9756790131354}");
    assert_eq!(hit.len(), 1, "NATO kept via the one matching descendant");
    let leaves = &hit[0].children[0].children;
    assert_eq!(leaves.len(), 1, "only the prefix-matching leaf survives");
    assert_eq!(leaves[0].id, rifleman_id);
    assert!(
        leaves[0].payload.is_some(),
        "the survivor is the placeable leaf"
    );

    // A broader classname prefix every US_Army leaf shares → all 8 back (folders by descent).
    let all = filter_catalog(&tree, "class:{");
    assert_eq!(
        all[0].children[0].children.len(),
        8,
        "all classnames share the GUID-brace start"
    );

    // CASE — the operand is matched case-insensitively against the id.
    let lower = filter_catalog(&tree, "class:{26a9756790131354}prefabs");
    assert_eq!(
        lower.len(),
        1,
        "lowercased operand matches the mixed-case id"
    );
    assert_eq!(lower[0].children[0].children[0].id, rifleman_id);

    // MISS — a prefix no classname starts with.
    assert!(
        filter_catalog(&tree, "class:{ZZZZ}").is_empty(),
        "a non-matching class prefix yields the empty tree"
    );
    // MISS — the operand is a PREFIX, not a substring: `Rifleman` sits mid-id, so it must NOT hit.
    assert!(
        filter_catalog(&tree, "class:Rifleman").is_empty(),
        "class: is prefix-only — a mid-classname token does not match"
    );

    // EMPTY-OPERAND — `class:` with nothing after matches nothing (the dock's empty state).
    assert!(
        filter_catalog(&tree, "class:").is_empty(),
        "class: with an empty operand matches nothing"
    );
    assert!(
        filter_catalog(&tree, "class:   ").is_empty(),
        "class: with whitespace-only operand also matches nothing"
    );

    // ADDITIVE PROOF — the label path is untouched: a plain query still self-matches the folder
    // and returns the historical full subtree (the `filter_catalog_rules` contract).
    assert_eq!(
        filter_catalog(&tree, "nato"),
        tree,
        "an un-prefixed query is still the T-055 label substring match"
    );
}

/// T-646 — the empty state distinguishes a mid-type `class:` (no operand yet) from a genuine
/// miss, so the dock "says so" rather than implying nothing matched.
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
    // A real miss (non-empty operand, or a label query) reads the plain noun message.
    assert_eq!(
        search_empty_message("class:zzz", "objects"),
        "No objects match."
    );
    assert_eq!(
        search_empty_message("rifleman", "assets"),
        "No assets match."
    );
}

// ── T-084 (RIGHT-SEARCH-002/003/004/005) — the grammar ───────────────────────────────────────

/// **THE DECISION THIS TICKET EXISTS TO MAKE** (wave-105 MINOR-2), pinned against a REAL
/// GUID-headed id from the committed catalogue —
/// `{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et`.
///
/// T-646's `class:` was a prefix over the WHOLE `resource_name`, and every Reforger resource
/// name starts with a GUID the author has never seen. So `class:<bare classname>` — the only
/// spelling an author actually knows — matched nothing and the tree silently emptied. This test
/// fires exactly there: it asserts the bare classname now HITS, and asserts (right beside it)
/// that T-646's GUID-path prefix still hits, because the fix is an OR, not a replacement.
#[test]
fn class_tail_matches_a_bare_classname_on_a_real_guid_headed_id() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let rifleman_id =
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";
    // Guard: this really is the shipped id, GUID head and `.et` tail included.
    assert_eq!(tree[0].children[0].children[0].id, rifleman_id);
    assert_eq!(
        classname_tail(rifleman_id),
        "Character_US_Rifleman",
        "the tail is the last path segment minus the extension"
    );

    // THE DEFECT, FIXED — a bare classname. Under T-646 this returned an EMPTY TREE.
    let bare = filter_catalog(&tree, "class:Character_US_Rifleman");
    assert_eq!(bare.len(), 1, "a bare classname must not empty the tree");
    assert_eq!(bare[0].children[0].children.len(), 1);
    assert_eq!(bare[0].children[0].children[0].id, rifleman_id);

    // A PARTIAL bare classname, as typed keystroke by keystroke.
    let partial = filter_catalog(&tree, "class:character_us_ri");
    assert_eq!(
        partial[0].children[0].children[0].id, rifleman_id,
        "tail matching is a prefix, and case-insensitive"
    );

    // T-646 UNREGRESSED — the full `resource_name` prefix still selects the same leaf.
    let guid = filter_catalog(&tree, "class:{26A9756790131354}Prefabs");
    assert_eq!(guid[0].children[0].children[0].id, rifleman_id);

    // The tail rule is a PREFIX, deliberately: a mid-classname token is still a miss…
    assert!(
        filter_catalog(&tree, "class:Rifleman").is_empty(),
        "tail matching is prefix-only, not substring"
    );
    // …and the grammar gives that query its own spelling instead.
    let globbed = filter_catalog(&tree, "class:*Rifleman");
    assert_eq!(
        globbed[0].children[0].children[0].id, rifleman_id,
        "a mid-classname token is reachable as a glob"
    );
}

/// RIGHT-SEARCH-003 — `mod:` filters by the ADDON, which in this catalogue is the root of the
/// category path and therefore the tree's depth-0 folder (`ArmaReforger/Vehicles/Wheeled/…`).
/// A hit keeps the root's whole subtree; a miss empties the tree. Both spellings of the operator
/// (`mod:` and the parity table's `mod `) are the same operator.
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
    // `mod:` is NOT a label search: `Wheeled` is a real folder, one level down, and must not
    // survive a mod query — otherwise the operator would just be a slower label match.
    assert!(
        filter_catalog(&tree, "mod:Wheeled").is_empty(),
        "mod: matches the addon root only, not any folder"
    );
    // Sanity: the label search over the same token still finds it.
    assert!(
        !filter_catalog(&tree, "Wheeled").is_empty(),
        "guard: `Wheeled` is a real folder the label search finds"
    );
    assert!(all_leaves > 0, "guard: the fixture tree has leaves");
}

/// RIGHT-SEARCH-004 — `*` (any run) and `?` (exactly one) matched WHOLE-STRING, over whichever
/// field the operator picked. Whole-string is the point: `US*` is starts-with, `*Medic` is
/// ends-with, `*ri*` is contains — three behaviours a bare token cannot express.
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

    // Crossed with `class:` — the same pattern engine, a different field. The tail arm makes
    // `*_MG` work without spelling out the GUID.
    let mg = filter_catalog(&tree, "class:*_MG");
    assert_eq!(leaves(&mg), ["US Machine Gunner"], "glob over the tail");
    let et = filter_catalog(&tree, "class:*Character_US_Medic.et");
    assert_eq!(
        leaves(&et),
        ["US Medic"],
        "glob over the full resource_name"
    );

    // Crossed with `mod:`.
    let vehicles = build_vehicle_catalog_tree(&vehicle_items());
    assert_eq!(
        filter_catalog(&vehicles, "mod:Arma*"),
        vehicles,
        "glob over the addon root"
    );
}

/// T-765 / wave-117 MINOR-1 — glob pattern literals and the haystack must share one fold.
/// `to_ascii_lowercase` on the pattern left `É` untouched while `hay.to_lowercase()` folded it
/// to `é`, so `CAFÉ*` refused `café_x`. Failure mode was a miss (safe), not a false positive.
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

/// RIGHT-SEARCH-005 — `/…/` is a regex over the selected field, unanchored (so `^`/`$` mean
/// something), case-insensitive, with alternation, classes and quantifiers. `{` is a LITERAL in
/// this subset — deliberately, because it opens every Reforger GUID.
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

    // Over the classname, where a regex earns its keep: `{` and `}` are literals, escaped or not.
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
    // `\d` over the real GUID heads: 7 of the 8 shipped BLUFOR ids open with a digit, the
    // Medic's `{C9E4FEAF…}` does not — a discriminator only real data provides.
    let digit_guid = filter_catalog(&tree, r"class:/^.\d/");
    assert_eq!(digit_guid[0].children[0].children.len(), 7);
    assert!(
        !leaves(&digit_guid).contains(&"US Medic".to_string()),
        "the one GUID that starts with a letter is excluded"
    );

    // Crossed with `mod:`.
    let vehicles = build_vehicle_catalog_tree(&vehicle_items());
    assert_eq!(
        filter_catalog(&vehicles, "mod:/^arma(reforger|3)$/"),
        vehicles,
        "regex over the addon root"
    );
}

/// A `/…/` body this engine cannot read is reported, not silently answered. Falling back to a
/// literal search for the regex text is the failure mode this ticket was opened on: an empty
/// tree that reads like "nothing matches" when the truth is "your pattern has a typo".
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
    // A lone slash is NOT a regex — an author searching a path fragment gets a literal search.
    assert_eq!(
        parse_search_query("/").pattern,
        SearchPattern::Plain("/".to_string())
    );
}

/// Every operator's mid-type state says what to type next. T-646 shipped this for `class:`; the
/// grammar generalises it, because a half-typed `mod:` or `//` is exactly as much "not a miss".
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
    // The genuinely empty query is still identity, not a mid-type state.
    assert_eq!(filter_catalog(&tree, "   "), tree);
}

/// The regex engine runs inside the wasm render loop on EVERY keystroke, so a catastrophic
/// backtracker must terminate rather than hang the tab. `(a+)+$` over a long non-matching
/// subject is the textbook exponential case; the step budget caps it.
#[test]
fn a_catastrophic_regex_terminates_on_the_step_budget() {
    let long = "a".repeat(40);
    let items = vec![character_row(
        &format!("{{Z}}Prefabs/{long}b.et"),
        &long,
        "NATO/US_Army/Long",
    )];
    let tree = build_catalog_tree(&items, "BLUFOR");
    // The answer itself is not the assertion — RETURNING is. Under an uncapped backtracker this
    // line never comes back.
    let _ = filter_catalog(&tree, "/^(a+)+$/");
    let _ = filter_catalog(&tree, "class:/(a|aa)+c/");
    // And a pattern the budget can afford still answers correctly.
    assert_eq!(filter_catalog(&tree, "/^a+$/").len(), 1);
}

/// Run `body` on a thread with the conventional wasm32 stack (1 MiB), which is the ONLY way
/// these next two tests mean anything: the default test-harness thread gets 2 MiB and the main
/// thread 8 MiB, so a rig run at the default size can pass while the shipped wasm build still
/// traps. Returns nothing, because there is nothing to return — a genuine stack overflow does
/// not unwind, it aborts the whole process, so the assertions *inside* `body` are the result.
fn on_a_wasm_sized_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(body)
        .expect("spawn")
        .join()
        .expect("the matcher must return, not abort");
}

/// **T-764 — the wave-117 MAJOR, and it was found by BUILDING A RIG, not by reading.** The
/// verifier lifted this engine verbatim onto a 1 MiB thread and proved that [`RX_BUDGET`] bounds
/// STEPS, NOT STACK DEPTH: [`RxCtx::node`] burns a native frame per step through the boxed
/// continuations, so four shapes ABORTED THE PROCESS rather than answering "no match" —
/// `(((…)))` at ~2500-3000, `^^^…` at ~20000-25000, `.?.?…` at ~3000, and `(x+x+)+y` over a
/// plain `x…` haystack at ~2700 haystack chars. In the browser that abort is a wasm trap that
/// kills Leptos and takes unsaved placements with it, so "the budget makes it return no-match"
/// was simply false for deep input.
///
/// This drives all four at (or past) those thresholds and asserts a CLEAN REFUSAL. Note which
/// bound catches which: the three long-PATTERN shapes never reach the matcher at all — they are
/// [`RX_MAX_PATTERN`] rejections at parse, i.e. the same `Invalid` the dock already explains —
/// while `(x+x+)+y` has a NINE-CHARACTER pattern and is caught only by [`RX_MAX_DEPTH`]. That
/// asymmetry is the reason a length cap cannot stand in for the depth cap.
///
/// It also pins the third case, which the bound's original sizing arithmetic assumed away: the
/// pattern that is *at* [`RX_MAX_PATTERN`] and so is NOT refused for length, `(`x512. It costs
/// one parser level per CHARACTER — 512 of them, the deepest this parser can be driven — and it
/// must still return rather than abort.
#[test]
fn deep_regex_input_refuses_instead_of_trapping_the_wasm_stack() {
    on_a_wasm_sized_stack(|| {
        // ── Long-pattern vectors: refused at parse, before a single frame is spent. ──
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
            // …and it arrives as the ordinary Invalid the operator already gets an explanation
            // for, not as a special new failure mode.
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

        // ── THE WORST CASE THE LENGTH CAP ADMITS, and the shape the old sizing arithmetic
        // assumed away. `(`x512 is exactly AT RX_MAX_PATTERN, so it is NOT refused for length
        // — it is parsed. Because nothing requires a pattern to balance, every one of those
        // 512 chars opens a parser level that no `)` closes, so this is `alt` → `seq` → `atom`
        // 512 deep: one level per CHARACTER, not per pair, which is the correction. Any deeper
        // input is refused by length, so this is the deepest RxParser::alt can ever go.
        //
        // `p.pos` is the evidence half here, the parser's answer to `depth_capped`: the parser
        // never backtracks and advances only in `atom`, one `(` per level entered, so a `pos`
        // of 512 after a FAILED parse proves all 512 levels were genuinely entered rather than
        // the parse bailing shallowly for some unrelated reason.
        let src: Vec<char> = "(".repeat(512).chars().collect();
        let mut p = RxParser { src: &src, pos: 0 };
        assert!(p.alt().is_none(), "unbalanced parens must not compile");
        assert_eq!(p.pos, 512, "the parse must have entered all 512 levels");
        // Same input through the real entry points: it must RETURN, not abort, and arrive as
        // the ordinary Invalid rather than as a new failure mode.
        assert!(
            Rx::parse(&"(".repeat(512)).is_none(),
            "512 unbalanced parens must be refused cleanly, not trap"
        );
        assert_eq!(
            parse_search_pattern(&format!("/{}/", "(".repeat(512))),
            SearchPattern::Invalid,
            "the deepest pattern the length cap admits must reach the dock as Invalid"
        );
        // One char further is refused for LENGTH, before the descent — that refusal is the
        // only reason 512 is the worst case rather than merely a case, so pin the constant
        // with it. Raising RX_MAX_PATTERN moves the worst case with it and invalidates the
        // margin in its doc comment; native debug aborts at 795 levels, so there is far less
        // room above 512 than the superseded "~256 levels / ~3x" arithmetic implied.
        assert!(
            Rx::parse(&"(".repeat(513)).is_none(),
            "one char past the cap must be refused for length"
        );
        assert_eq!(
            RX_MAX_PATTERN, 512,
            "re-measure the abort floor before changing this — see RX_MAX_PATTERN's doc"
        );

        // ── The depth vectors: patterns the length cap ACCEPTS that still recurse past the
        // bound. `depth_capped` is the load-bearing half of each assertion — it is latched only
        // by RX_MAX_DEPTH actually turning a branch away, so a `false` answer here cannot be
        // mistaken for a pattern that merely failed shallowly.
        //
        // `^`x512 is exactly at RX_MAX_PATTERN and every caret is a zero-width node, so it
        // recurses 512 deep on a 3-char subject: depth with no work at all, the shape the step
        // budget is blindest to.
        let carets = Rx::parse(&"^".repeat(512)).expect("512 carets is within the length cap");
        assert_eq!(carets.search("abc"), (false, true));

        // THE VECTOR NO LENGTH CAP CAN SEE: nine characters of pattern, and the depth comes
        // from the HAYSTACK. 3000 chars is past the verifier's ~2700 abort threshold.
        let evil = Rx::parse("(x+x+)+y").expect("the classic exponential pattern parses");
        assert_eq!(evil.search(&"x".repeat(3000)), (false, true));
        // Same pattern, same 3000-char subject, through the real dock entry point.
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

/// The other half of the bound: 400 must be a cap on ABUSE, not on the catalogue. If
/// [`RX_MAX_DEPTH`] were set anywhere near real query depth this would go red, which is what
/// stops a future "just lower it to be safe" from silently emptying the tree.
#[test]
fn honest_catalogue_patterns_stay_far_under_the_depth_cap() {
    on_a_wasm_sized_stack(|| {
        // The longest `resource_name` shape the shipped catalogue actually contains (95 chars
        // in the golden fixture), padded past it for margin.
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
        // And a deeply-but-legally nested pattern under the length cap still ANSWERS rather
        // than refusing: 200 levels of nesting is well inside a 400-level bound.
        let nested = Rx::parse(&format!("{}a{}", "(".repeat(200), ")".repeat(200)))
            .expect("401 chars is within the length cap");
        assert_eq!(nested.search("a"), (true, false));
    });
}

/// Wave-105 verifier BLOCKER-1: `strip_prefix_ci` byte-sliced `s[..6]` with no char-boundary
/// check, so any query whose 6th byte split a multibyte char — "beauté", "a日本" — panicked on
/// the keystroke and aborted the wasm runtime. Every dock search routes every keystroke here.
/// All prior tests were ASCII, which is why nothing covered it.
#[test]
fn multibyte_queries_do_not_panic_the_recogniser() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    // 6th byte mid-'é' (é is 2 bytes at byte 5): the exact boundary-split shape.
    for q in [
        "beauté",
        "abcdeß",
        "a日本",
        "clasé",
        "über-search",
        "日本語のクエリ",
    ] {
        // Must not panic; multibyte text is never the `class:` operator, so it label-matches.
        let _ = filter_catalog(&tree, q);
        assert_eq!(
            parse_search_query(q).field,
            SearchField::Label,
            "{q} must stay Label"
        );
    }
    // A multibyte OPERAND after a well-formed `class:` head must also survive.
    let _ = filter_catalog(&tree, "class:beauté");
}

/// The single load-bearing assertion, wired to FIRE once: `class:` selects a leaf by its
/// classname where a plain label query over the SAME token cannot. If the recogniser were
/// dropped (the query fell through to the label path) the classname-only token would find nothing
/// and this would fail — so a GREEN here means the `class:` arm actually ran.
#[test]
fn class_prefix_fires_where_label_cannot() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    // `Character_US_Rifleman` is in every leaf's classname (`id`) but in NO label
    // (labels are "US Rifleman", "US Grenadier", …), so it is the perfect discriminator.
    let token = "Character_US_Rifleman";

    // Label path over the token: the historical matcher finds nothing (it is not in any label).
    assert!(
        filter_catalog(&tree, token).is_empty(),
        "guard: the classname token is absent from every label"
    );
    // class: path over the same token: the recogniser routes to id-prefix matching. It is a
    // prefix of the id only after the `{GUID}Prefabs/…/` head, so match on the full leading id.
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

/// CHIP + SEARCH COMPOSITION — the active chip filters the tree (via `build_catalog_tree`, which
/// side-filters through `character_matches_eden_side`) BEFORE `class:`/label search runs on the
/// result. An OPFOR chip + a BLUFOR-classname `class:` query is empty (the BLUFOR leaves were
/// already dropped by the chip), while the same query on the BLUFOR tree hits — proving the two
/// filters compose in that order.
#[test]
fn chip_side_then_class_search_compose() {
    let mut items = golden_items();
    items.push(character_row(
        "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
        "USSR Rifleman",
        "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
    ));

    // Chip = OPFOR → the tree holds only USSR. A BLUFOR-classname class: query finds nothing.
    let opfor = build_catalog_tree(&items, "OPFOR");
    assert!(
        filter_catalog(&opfor, "class:{26A9756790131354}").is_empty(),
        "chip filtered BLUFOR out before search; the BLUFOR class prefix cannot match"
    );
    // …but the OPFOR classname does match on the OPFOR tree.
    let opfor_hit = filter_catalog(&opfor, "class:{DCB41B3746FDD1BE}");
    assert_eq!(
        opfor_hit.len(),
        1,
        "the OPFOR class prefix matches the USSR leaf"
    );

    // Chip = BLUFOR → the same BLUFOR class query now hits (chip kept the NATO leaves).
    let blufor = build_catalog_tree(&items, "BLUFOR");
    assert_eq!(
        filter_catalog(&blufor, "class:{26A9756790131354}").len(),
        1,
        "on the BLUFOR tree the BLUFOR class prefix hits — search runs on the chip-filtered tree"
    );
    // Composition also holds for a plain label query on the chip-filtered tree.
    assert!(
        filter_catalog(&opfor, "US Rifleman").is_empty(),
        "a label query for a BLUFOR role is empty under the OPFOR chip"
    );
}

/// CHIP PREDICATE PER SIDE — `character_matches_eden_side` (the predicate RIGHT-SUBMODE-001 rides,
/// and what `build_catalog_tree` filters through) admits a row for exactly its own side. This
/// pins the predicate the chip filtering depends on, independently of the tree builder.
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
    // Each row matches its own side and no other.
    assert!(character_matches_eden_side(&us, "BLUFOR"));
    assert!(!character_matches_eden_side(&us, "OPFOR"));
    assert!(!character_matches_eden_side(&us, "INDFOR"));
    assert!(character_matches_eden_side(&ussr, "OPFOR"));
    assert!(!character_matches_eden_side(&ussr, "BLUFOR"));
    assert!(character_matches_eden_side(&fia, "INDFOR"));
    assert!(!character_matches_eden_side(&fia, "BLUFOR"));
    // Unknown / empty side never matches (the chip row admits only the three sides).
    assert!(!character_matches_eden_side(&us, "CIV"));
    assert!(!character_matches_eden_side(&us, ""));
}
