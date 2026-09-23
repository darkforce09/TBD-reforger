//! Evaluation of `cfg` predicates.
//!
//! **Role:** decides whether the predicate of a `#[cfg(…)]` attribute or a `cfg!(…)` macro holds
//! in every build, fails in every build, or depends on the build.
//! **Position:** second stage of the scrub pipeline, between the lexer and the block-level passes.
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** an undecidable predicate is `None`, never a default. `any` with no arms is
//! false and `all` with no arms is true, matching the compiler's own rule.

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

/// Truth of a `cfg` predicate for **any** build.
///
/// Only the combinators decide: `any` with no arms is false and `all` with no arms is true,
/// matching rustc — which is what makes `#[cfg(any())]` the canonical never-compiled attribute.
/// Every atom (`target_arch = "wasm32"`, `feature = "x"`, a bare identifier) is build-dependent,
/// so `None` means the predicate depends on the build and a caller must leave such an item alone:
/// `target_arch = "wasm32"` and `feature = "x"` guard real production code.
pub(crate) fn cfg_eval(pred: &str) -> Option<bool> {
    let p = pred.trim();
    if p.is_empty() {
        return None;
    }
    for name in ["any", "all"] {
        if let Some(args) = call_args(p, name) {
            let vals: Vec<Option<bool>> = split_top_commas(&args)
                .iter()
                .map(|s| cfg_eval(s))
                .collect();
            return if name == "any" {
                if vals.contains(&Some(true)) {
                    Some(true)
                } else if vals.iter().all(|v| *v == Some(false)) {
                    Some(false) // includes `any` — no arm is true
                } else {
                    None
                }
            } else if vals.contains(&Some(false)) {
                Some(false)
            } else if vals.iter().all(|v| *v == Some(true)) {
                Some(true) // includes `all` — no arm is false
            } else {
                None
            };
        }
    }
    if let Some(args) = call_args(p, "not") {
        return cfg_eval(&args).map(|b| !b);
    }
    None
}
