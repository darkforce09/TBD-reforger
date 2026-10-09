//! Role: pins that the picking adapter resolves picks and marquees through the document's mixed
//! slot-and-vehicle queries, read from `picking.rs`'s own source, and the scrubber that makes those
//! reads honest.
//! Position: a test module of `mission_editing_session`, mounted from `picking.rs`.
//! Signals & state: none; pure text checks.
//! Invariants: every source is scrubbed of comments and literals before a token is looked for, so
//! a mention of a call in prose cannot satisfy a pin.

use source_scrub::strip_rust_lexical_noise;

#[path = "source_scrub.rs"]
mod source_scrub;

#[test]
fn rust_lexical_scrubber_eats_comments_and_literals_but_not_code() {
    let out = strip_rust_lexical_noise(
        "// core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);\nlet keep = 1;\n",
    );
    assert!(
        !out.contains("move_entities_and_vehicles"),
        "a line comment must not survive: {out:?}"
    );
    assert!(
        out.contains("let keep = 1;"),
        "live code must survive: {out:?}"
    );

    for (label, src) in [
        (
            "doc comment",
            "/// see move_entities_and_vehicles\nlet keep = 1;",
        ),
        ("inner doc", "//! move_entities_and_vehicles\nlet keep = 1;"),
        (
            "block comment",
            "/* move_entities_and_vehicles */ let keep = 1;",
        ),
        (
            "nested block",
            "/* outer /* move_entities_and_vehicles */ still */ let keep = 1;",
        ),
        (
            "string literal",
            "let s = \"move_entities_and_vehicles\"; let keep = 1;",
        ),
        (
            "escaped string",
            "let s = \"\\\"move_entities_and_vehicles\\\"\"; let keep = 1;",
        ),
        (
            "raw string",
            "let s = r\"move_entities_and_vehicles\"; let keep = 1;",
        ),
        (
            "hashed raw string",
            "let s = r##\"move_entities_and_vehicles \"# \"##; let keep = 1;",
        ),
        (
            "byte string",
            "let s = b\"move_entities_and_vehicles\"; let keep = 1;",
        ),
    ] {
        let out = strip_rust_lexical_noise(src);
        assert!(
            !out.contains("move_entities_and_vehicles"),
            "{label} must not survive scrubbing: {out:?}"
        );
        assert!(
            out.contains("let keep = 1;"),
            "{label}: live code lost: {out:?}"
        );
    }

    let life = strip_rust_lexical_noise("fn f<'a>(x: &'a str, c: char) { let q = '\\''; }");
    assert!(
        life.contains("fn f<'a>(x: &'a str, c: char)"),
        "lifetimes kept: {life:?}"
    );
    assert!(!life.contains("'\\''"), "the char literal went: {life:?}");

    let tricky = strip_rust_lexical_noise("let s = \"// /*\"; call_me();");
    assert!(
        tricky.contains("call_me();"),
        "string-borne `//` must not eat code: {tricky:?}"
    );

    let src = "// a\nlet keep = 1;\n";
    let out = strip_rust_lexical_noise(src);
    assert_eq!(
        out.chars().count(),
        src.chars().count(),
        "char count preserved"
    );
    assert_eq!(
        out.lines().count(),
        src.lines().count(),
        "newlines preserved: {out:?}"
    );
}

#[test]
fn picking_adapter_names_the_mixed_pick_and_marquee_api() {
    // The selection tool's pick and marquee forward to the picking adapter, which is the one
    // place that joins the spatial queries to the document's mixed resolution.
    let select = strip_rust_lexical_noise(include_str!("../picking.rs"));
    assert!(
        select.contains("MissionDocCore::pick_slot_or_vehicle("),
        "the picking adapter has no `MissionDocCore::pick_slot_or_vehicle(` call token outside \
             comments/strings — the mixed pick was forked or deleted"
    );
    assert!(
        select.contains("MissionDocCore::marquee_ids_with_vehicles("),
        "the picking adapter has no `MissionDocCore::marquee_ids_with_vehicles(` call token \
             outside comments/strings — the mixed marquee was forked or deleted"
    );
}
