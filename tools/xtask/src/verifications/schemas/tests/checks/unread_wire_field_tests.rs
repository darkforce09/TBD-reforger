use super::{
    UNREAD_WIRE_FIELDS, count_mod_readers, repo_root, strip_enfusion_comments_and_strings,
    unread_wire_field_failures,
};
use std::fs;
use std::path::PathBuf;

fn mod_root() -> PathBuf {
    repo_root()
        .expect("repo root")
        .join("apps/mod/tbd-framework")
}

/// Green on the live tree: every remaining row is at its pinned baseline. This is the assertion
/// the gate makes; if it reds here, a reader landed (or an unrelated identifier moved) and the
/// row must be retired or deliberately re-pinned.
///
/// The name is kept for the CI logs and the wave verify notes that grep for it, but "unread" is
/// no longer true of every row: `objectives`, `seats`, `size`, `shape`, `area` and `gadgets`
/// each pin a MIXTURE now — an unrelated pre-existing identifier plus, for some, a landed
/// reader's own hits. See `UNREAD_WIRE_FIELDS` for which is which.
#[test]
fn all_1_3_fields_are_unread_on_the_live_tree() {
    let f = unread_wire_field_failures(&mod_root()).expect("scan mod tree");
    assert!(
        f.is_empty(),
        "a 1.3 wire field is no longer unread:\n  {f:#?}"
    );
}

/// The fire-once proof, MEASURED, not assumed: without it, "asserts the baseline" could be a
/// check that never notices anything at all.
///
/// It used to plant one `framing` identifier in a scratch tree and watch a CLEAN 0 baseline
/// trip. That reader landed and the row retired, and no clean-0 row is left to stand in
/// for it — so the proof is rebuilt on `objectives`, and in BOTH directions, which is strictly
/// stronger than the version it replaces:
///
///   * a scratch tree holding EXACTLY the pinned count reports no failure (the assertion is
///     not simply always-red, which a one-directional test cannot rule out);
///   * one more identifier — the shape a new reader has, since `JsonLoadContext` binds by
///     member name and no reader can exist without spelling the key — trips it, by name and
///     with the reason its baseline is nonzero.
#[test]
fn unread_gate_fires_when_a_reader_appears() {
    let pinned = UNREAD_WIRE_FIELDS
        .iter()
        .find(|f| f.name == "objectives")
        .expect("the objectives row must exist");

    let dir = std::env::temp_dir().join(format!("t706-unread-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let scripts = dir.join("Scripts/Game/TBD/Gamemode");
    fs::create_dir_all(&scripts).expect("scratch mod tree");
    let reader = scripts.join("TBD_FutureObjectiveReader.c");

    // A plausible reader: struct members bound by JsonLoadContext (which maps by name).
    let at_baseline =
        "class TBD_FutureObjectiveStruct { string objectives; }\n".repeat(pinned.expected);
    fs::write(&reader, &at_baseline).expect("write baseline");
    assert_eq!(
        count_mod_readers(&dir, "objectives").expect("count"),
        pinned.expected,
        "the scratch tree must sit exactly on the pinned baseline"
    );
    assert!(
        !unread_wire_field_failures(&dir)
            .expect("scan scratch")
            .iter()
            .any(|m| m.contains("'objectives'")),
        "a tree AT the baseline must not be reported — otherwise the assertion is vacuous"
    );

    // One more identifier is what a newly landed reader looks like.
    fs::write(
        &reader,
        format!("{at_baseline}class TBD_OneMore {{ string objectives; }}\n"),
    )
    .expect("write reader");
    assert_eq!(
        count_mod_readers(&dir, "objectives").expect("count"),
        pinned.expected + 1,
        "an objectives identifier in a .c file must be counted as a reader"
    );
    let f = unread_wire_field_failures(&dir).expect("scan scratch");
    assert!(
        f.iter()
            .any(|m| m.contains("'objectives'") && m.contains(pinned.why)),
        "the gate must fail and name objectives + its baseline reason once a reader appears; got {f:#?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The stripper must ignore the field name inside a `//` comment and inside a `""` string, or
/// prose like the word "behaviour" would count as a reader (MEASURED before stripping: it did).
#[test]
fn comments_and_string_literals_do_not_count_as_readers() {
    let dir = std::env::temp_dir().join(format!("t706-strip-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch");
    fs::write(
        dir.join("TBD_ProseOnly.c"),
        "//! combatMode is discussed here in prose only.\n\
         void F() { string s = \"combatMode goes on the wire\"; /* combatMode again */ }\n",
    )
    .expect("write prose");
    assert_eq!(
        count_mod_readers(&dir, "combatMode").expect("count"),
        0,
        "combatMode only in a comment and a string literal must count as ZERO readers"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A direct unit test of the stripper on all three shapes at once.
#[test]
fn stripper_removes_line_block_and_string_bodies() {
    let src = "a //b\nc /* d */ e\nf \"g h\" i\n";
    let out = strip_enfusion_comments_and_strings(src);
    for gone in ["b", "d", "g", "h"] {
        assert!(
            !out.contains(gone),
            "{gone} should be stripped from {out:?}"
        );
    }
    for kept in ["a", "c", "e", "f", "i"] {
        assert!(out.contains(kept), "{kept} should survive in {out:?}");
    }
}

// Every collision baseline (>0) must name the pre-existing identifier it is pinning, so a future
/// reader cannot silently re-use a fat baseline as cover. A `clean`/`stripped`/`substring` note
/// is only allowed at baseline 0.
#[test]
fn nonzero_baselines_explain_the_pre_existing_identifier() {
    for f in UNREAD_WIRE_FIELDS {
        if f.expected > 0 {
            assert!(
                !f.why.is_empty()
                    && (f.why.contains("unrelated")
                        || f.why.contains("different")
                        || f.why.contains("existing")),
                "'{}' baseline {} must explain the unrelated identifier it pins (got: {:?})",
                f.name,
                f.expected,
                f.why
            );
        }
    }
}
