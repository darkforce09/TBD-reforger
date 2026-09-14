//! Evaluation of `cfg` predicates, and the rewrite that resolves them for the shipping build.
//!
//! **Role:** decides whether a `#[cfg(…)]` attribute keeps or removes the item it annotates, and
//! rewrites an item as the wasm32 build would see it.
//! **Position:** second stage of the scrub pipeline, between the lexer and the block-level passes.
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** an undecidable predicate is `None`, never a default. `any` with no arms is
//! false and `all` with no arms is true, matching the compiler's own rule.

use super::lexer::{balanced, blank, is_ident_char, mask};
use super::scrub::item_end_after;

/// The argument text of `s` when `s` is exactly `name( … )`, otherwise `None`.
///
/// The name match is word-bounded, so `cfg_attr(…)` does not strip as `cfg`.
pub(super) fn call_args(s: &str, name: &str) -> Option<String> {
    let t = s.trim();
    let rest = t.strip_prefix(name)?.trim_start();
    let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
    let mut d = 0i32;
    for ch in inner.chars() {
        match ch {
            '(' => d += 1,
            ')' => {
                d -= 1;
                if d < 0 {
                    return None; // the ')' we stripped was not the matching one
                }
            }
            _ => {}
        }
    }
    (d == 0).then(|| inner.to_string())
}

/// Split `s` on the commas that are not inside a nested delimiter group. Empty input yields no
/// arms rather than one empty arm.
fn split_top_commas(s: &str) -> Vec<String> {
    if s.trim().is_empty() {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut d = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' | '[' | '{' => d += 1,
            ')' | ']' | '}' => d -= 1,
            ',' if d == 0 => {
                parts.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts
}

/// Statically decidable truth of a `cfg` predicate, with `leaf` deciding the atoms
/// (`target_arch = "wasm32"`, `feature = "x"`, a bare identifier).
///
/// `any` with no arms is false and `all` with no arms is true, matching rustc — which is what
/// makes `#[cfg(any())]` the canonical never-compiled attribute. `None` means the predicate is
/// build-dependent.
fn cfg_eval_with(pred: &str, leaf: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
    let p = pred.trim();
    if p.is_empty() {
        return None;
    }
    for name in ["any", "all"] {
        if let Some(args) = call_args(p, name) {
            let vals: Vec<Option<bool>> = split_top_commas(&args)
                .iter()
                .map(|s| cfg_eval_with(s, leaf))
                .collect();
            return if name == "any" {
                if vals.iter().any(|v| *v == Some(true)) {
                    Some(true)
                } else if vals.iter().all(|v| *v == Some(false)) {
                    Some(false) // includes `any` — no arm is true
                } else {
                    None
                }
            } else if vals.iter().any(|v| *v == Some(false)) {
                Some(false)
            } else if vals.iter().all(|v| *v == Some(true)) {
                Some(true) // includes `all` — no arm is false
            } else {
                None
            };
        }
    }
    if let Some(args) = call_args(p, "not") {
        return cfg_eval_with(&args, leaf).map(|b| !b);
    }
    leaf(p)
}

/// Truth of a `cfg` predicate for **any** build.
///
/// `None` means build-dependent, and a caller must leave such an item alone: `target_arch =
/// "wasm32"` and `feature = "x"` guard real production code.
pub(crate) fn cfg_eval(pred: &str) -> Option<bool> {
    cfg_eval_with(pred, &|_| None)
}

/// Truth of a `cfg` predicate **for the wasm32 build** — the one that actually ships.
///
/// Only `target_arch` is decided; every other atom stays unknown, which callers must treat as a
/// refusal rather than as a default.
pub(crate) fn cfg_eval_wasm(pred: &str) -> Option<bool> {
    cfg_eval_with(pred, &|atom| {
        let (k, v) = atom.split_once('=')?;
        (k.trim() == "target_arch").then(|| v.trim().trim_matches('"') == "wasm32")
    })
}

/// True when `src` contains any whole-word identifier from the `cfg` family — `cfg`,
/// `cfg_attr`, `cfg_match`, or a future sibling.
///
/// The prefix rule is deliberate: recognising only the exact spelling `cfg` would miss the next
/// variant, which is the class of miss this whole module exists to avoid.
pub(crate) fn mentions_cfg_family(src: &str) -> bool {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < c.len() {
        if is_ident_char(c[i]) && (i == 0 || !is_ident_char(c[i - 1])) {
            let s = i;
            while i < c.len() && is_ident_char(c[i]) {
                i += 1;
            }
            let w: String = c[s..i].iter().collect();
            if w == "cfg" || w.starts_with("cfg_") {
                return true;
            }
            continue;
        }
        i += 1;
    }
    false
}

/// Rewrite `item` as the wasm32 build sees it: a `cfg` that is true there loses its attribute
/// and keeps its item, a `cfg` that is false there loses attribute and item together.
///
/// # Panics
///
/// On any `cfg` this pass cannot decide — a `feature` gate, a bare identifier, a `cfg_attr`. An
/// undecidable gate would let the harness and the shipped build run different programs, so it is
/// reported rather than guessed at. The final assertion is the belt: no `cfg` of any spelling
/// survives into the returned text.
pub(crate) fn resolve_wasm_cfg(item: &str) -> String {
    let chars: Vec<char> = item.chars().collect();
    let scan = mask(&chars, true);
    let mut out = chars.clone();
    let mut i = 0usize;
    while i < scan.len() {
        if scan[i] != '#' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if scan.get(j) == Some(&'!') {
            j += 1;
        }
        if scan.get(j) != Some(&'[') {
            i += 1;
            continue;
        }
        let Some(close) = balanced(&scan, j, '[', ']') else {
            i += 1;
            continue;
        };
        // Literals intact here: the predicate is `target_arch = "wasm32"`.
        let inner: String = chars[j + 1..close].iter().collect();
        if let Some(pred) = call_args(&inner, "cfg") {
            match cfg_eval_wasm(&pred) {
                Some(true) => {
                    for k in i..=close {
                        out[k] = blank(out[k]);
                    }
                }
                Some(false) => {
                    let end = item_end_after(&scan, close + 1);
                    for k in i..end {
                        out[k] = blank(out[k]);
                    }
                }
                None => panic!(
                    "`#[cfg({pred})]` inside a cure-1 pinned item cannot be resolved \
                     for the wasm32 build. This pin compiles and runs the item to prove the \
                     path is live, so a gate the harness cannot decide would let it run a \
                     different program from the one that ships. Move the conditional out of \
                     the pinned item, or teach `cfg_eval_wasm` the atom."
                ),
            }
        }
        i = close + 1;
    }
    let resolved: String = out.into_iter().collect();
    assert!(
        !mentions_cfg_family(&resolved),
        "conditional compilation survived resolution:\n{resolved}"
    );
    resolved
}
