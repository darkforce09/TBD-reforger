//! The source probe of the slot attribute existence check.
//!
//! **Role:** proves `slot_attrs_exists` answers from the raw slot map, never from a materialised
//! view that drops hidden slots.
//! **Position:** a document operations test, mounted from the operations module; it reads
//! `entity/identity.rs` through the map engine's source scrub.
//! **Signals & state:** none; reads one source file at compile time.
//! **Invariants:** the probe finds exactly one `fn slot_attrs_exists(` in the scrubbed source.

use crate::source_scrub::strip_rust_lexical_noise;

/// The brace-balanced body of the one function `marker` opens in the scrubbed source `src`.
fn only_fn_body(src: &str, marker: &str) -> String {
    let hits = src.matches(marker).count();
    assert_eq!(
        hits, 1,
        "expected exactly one `{marker}` in the scrubbed source, found {hits} — 0 means it was \
             renamed or deleted, 2+ means a shadow definition; either way this probe cannot examine \
             code it cannot unambiguously find"
    );
    let at = src.find(marker).expect("counted exactly one");
    let open = at + src[at..].find('{').expect("a fn has a body");
    let mut depth = 0usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return src[open..=open + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces after `{marker}`");
}

#[test]
fn slot_attrs_exists_reads_the_raw_map_not_the_materialized_view() {
    let ops = strip_rust_lexical_noise(include_str!("../entity/identity.rs"));
    let body = only_fn_body(&ops, "fn slot_attrs_exists(");
    assert!(
        body.len() > 2,
        "T-937.3: the scrubbed body of slot_attrs_exists is empty — the probe is reading \
             nothing and would pass over anything"
    );
    assert!(
        body.contains("slot_exists("),
        "T-937.3: slot_attrs_exists must answer existence off the raw slot map \
             (`MissionDocCore::slot_exists`), so a hidden slot counts as existing; body:{body}"
    );
    assert!(
        !body.contains("materialize("),
        "T-937.3: slot_attrs_exists must NOT materialize — the SoA drops hidden slots \
             (T-665/T-701), so a materialize-sourced existence check answers NO for a slot the \
             document holds, and it pays an O(all slots) walk to do it; body:{body}"
    );
}
