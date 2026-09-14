//! The compile-time constants of a file: which names the compiler fixes, and which of those this
//! pass managed to fold.
//!
//! **Role:** harvests every `const`, `static` and `let` binding textually, folds what it can to a
//! fixpoint, and records the rest as constant-but-unread.
//! **Position:** feeds the expression folder and, through it, every block-removal pass.
//! **Signals & state:** none; the harvest is a pure function of the source text.
//! **Invariants:** a name that is compile-time by Rust's rules but that this pass could not fold
//! stays in `opaque`, so conditions built on it fail closed instead of being reported as live.

use std::collections::{HashMap, HashSet};

use super::expr::{eval_value, fold_cfg_macros, lex, Tok, Val};
use super::lexer::{is_ident_char, kw_at};

/// Cast targets, so that `x as u8` does not read as a reference to a runtime identifier.
const PRIMITIVE_TYPES: &[&str] = &[
    "bool", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128",
    "usize", "f32", "f64",
];

/// What the compiler decides about a file's names, as opposed to what the program computes.
///
/// The split between the two fields is the fail-closed mechanism. `known` is what this pass
/// managed to fold. `opaque` is the set of names that are compile-time constant by Rust's own
/// rules — every `const` and `static`, plus a `let` whose initialiser is made only of
/// compile-time material — which this pass could **not** fold. A condition gated on an `opaque`
/// name had its truth fixed at compile time and this evaluator failed to read it, which is exactly
/// the case that must not be reported as live code.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Consts {
    /// Names this pass folded, with the value it folded them to.
    pub(super) known: HashMap<String, Val>,
    /// Names the compiler fixes that this pass could not fold.
    opaque: HashSet<String>,
}

impl Consts {
    fn is_compile_time(&self, name: &str) -> bool {
        let last = name.rsplit("::").next().unwrap_or(name);
        PRIMITIVE_TYPES.contains(&name)
            || matches!(last, "black_box" | "identity")
            || self.known.contains_key(name)
            || self.opaque.contains(name)
    }
}

/// True when `expr` is built only out of material the compiler decides: literals, operators, and
/// identifiers that `Consts` recognises as compile-time.
///
/// This is the predicate that lets the scrubber fail closed without deleting the program. A
/// condition that mentions a runtime name is genuinely conditional and must be left alone. A
/// condition that mentions none is a compile-time constant whatever its shape, so if `eval_bool`
/// could not fold it the failure is the evaluator's and the block is treated as possibly dead.
///
/// Note what is deliberately not enumerated: the operators. A token the lexer does not model does
/// not disqualify an expression from being constant-shaped, which is what makes the predicate
/// closed under shapes nobody has written yet.
pub(super) fn constant_shaped(expr: &str, consts: &Consts) -> bool {
    let toks = lex(&fold_cfg_macros(expr));
    !toks.is_empty()
        && toks.iter().all(|t| match t {
            Tok::Ident(name) => consts.is_compile_time(name),
            _ => true,
        })
}

/// One `const`, `static` or `let` binding site, harvested textually.
struct Binding {
    name: String,
    expr: String,
    compile_time: bool,
    trusted: bool,
}

/// Every `const NAME[: T] = …;`, `static …` and `let …` binding in `scan`, in source order.
///
/// Deliberately conservative: `mut` bindings are skipped because they can be reassigned out of
/// sight. The pass is not scope-aware, so its failure direction is a false strip — which turns a
/// source pin red and loud rather than green and silent.
///
/// The cursor advances one keyword at a time rather than jumping to the end of each initialiser.
/// Jumping is correct for finding the next sibling binding and wrong for anything nested: a
/// `let run = async { … };` or a `let send = move |t| { … };` would swallow its whole body, hiding
/// every binding inside it.
fn binding_sites(scan: &[char]) -> Vec<Binding> {
    let mut sites: Vec<Binding> = Vec::new();
    let n = scan.len();
    let mut i = 0usize;
    while i < n {
        let Some(kw) = ["const", "static", "let"]
            .iter()
            .find(|k| kw_at(scan, i, k))
            .copied()
        else {
            i += 1;
            continue;
        };
        let mut j = i + kw.len();
        while j < n && scan[j].is_whitespace() {
            j += 1;
        }
        if kw_at(scan, j, "mut") {
            i += kw.len();
            continue; // reassignable — out of scope for a text pass
        }
        let s = j;
        while j < n && is_ident_char(scan[j]) {
            j += 1;
        }
        if j == s {
            i += kw.len();
            continue;
        }
        let name: String = scan[s..j].iter().collect();
        while j < n && scan[j].is_whitespace() {
            j += 1;
        }
        let compile_time = kw != "let";
        let mut annotated = false;
        if scan.get(j) == Some(&':') {
            j += 1;
            while j < n && scan[j].is_whitespace() {
                j += 1;
            }
            let ts = j;
            while j < n && is_ident_char(scan[j]) {
                j += 1;
            }
            let ty: String = scan[ts..j].iter().collect();
            // A non-`bool` `let` annotation is a runtime binding this pass has no business
            // folding. A non-`bool` `const`/`static` is still compile-time, and folding its
            // number is what keeps `const LIMIT: usize = 5; if LIMIT > 3` out of the
            // fail-closed path.
            if ty != "bool" && !compile_time {
                i += kw.len();
                continue;
            }
            annotated = ty == "bool";
            while j < n && scan[j].is_whitespace() {
                j += 1;
            }
        }
        if scan.get(j) != Some(&'=') || scan.get(j + 1) == Some(&'=') {
            i += kw.len();
            continue;
        }
        j += 1;
        let es = j;
        let mut d = 0i32;
        while j < n {
            match scan[j] {
                '(' | '[' | '{' => d += 1,
                ')' | ']' | '}' => d -= 1,
                ';' if d <= 0 => break,
                _ => {}
            }
            j += 1;
        }
        let expr: String = scan[es..j.min(n)].iter().collect();
        let trusted = compile_time || annotated || matches!(expr.trim(), "true" | "false");
        sites.push(Binding {
            name,
            expr,
            compile_time,
            trusted,
        });
        // One keyword forward, NOT to the end of the initializer — see the note above.
        i += kw.len();
    }
    sites
}

/// How many rounds of constant-to-constant substitution to run. A chain of `const` definitions
/// needs one round per link and the links may appear in any order, but real chains are two or three
/// long and an unbounded loop inside a test harness would be its own defect.
const CONST_FOLD_ROUNDS: usize = 8;

/// The compile-time constants of `scan`: what folded, and what provably did not.
///
/// Folding iterates to a fixpoint because one `const` may be defined in terms of another, in
/// either source order. Names that did not fold are kept in `opaque` rather than dropped: a `const`
/// or `static` is compile-time by definition, so one this pass cannot read is a constant it failed
/// to read, not a runtime value. A `let` earns the same treatment only when its initialiser is
/// itself made of compile-time material, because scrubbing a block guarded by a genuine runtime
/// `let` would delete the program.
pub(super) fn constants(scan: &[char]) -> Consts {
    let sites = binding_sites(scan);
    let mut consts = Consts {
        known: HashMap::new(),
        opaque: sites
            .iter()
            .filter(|b| b.compile_time)
            .map(|b| b.name.clone())
            .collect(),
    };
    for _ in 0..CONST_FOLD_ROUNDS {
        let mut round: HashMap<String, Option<Val>> = HashMap::new();
        for b in &sites {
            let v = b.trusted.then(|| eval_value(&b.expr, &consts)).flatten();
            // A name bound twice to different values tells this pass nothing it can use.
            round
                .entry(b.name.clone())
                .and_modify(|e| {
                    if *e != v {
                        *e = None;
                    }
                })
                .or_insert(v);
        }
        let known: HashMap<String, Val> = round
            .into_iter()
            .filter_map(|(k, v)| v.map(|x| (k, x)))
            .collect();
        if known == consts.known {
            break;
        }
        consts.known = known;
    }
    // A `let` whose initialiser mentions nothing the program computes is a constant wearing a
    // `let`: `let w: bool = (true, false).1;` must not launder a dead block into a live one.
    for b in &sites {
        if !consts.known.contains_key(&b.name) && constant_shaped(&b.expr, &consts) {
            consts.opaque.insert(b.name.clone());
        }
    }
    let known = std::mem::take(&mut consts.known);
    consts.opaque.retain(|n| !known.contains_key(n));
    consts.known = known;
    consts
}
