//! Attack battery for the source scrubber: every shape a guard needle could hide in.

/// The scrubber's own guard: every shape a needle could hide in, fed through and required to
/// come out empty.
///
/// A scrubber that quietly stopped scrubbing would leave every guard built on it hollow while
/// all of them stayed green, so the tool that exists to stop silent passes gets a test rather
/// than a comment.
///
/// The cases run in tiers: text decoys (comments and string literals), dead-code wrappers
/// (`if false`, `while false`, `#[cfg(any())]`, a `match` guard, a `const` folded to false, a
/// `black_box(false)`, code after a `return;`, and a shadow copy parked in an uncompiled
/// module), spelling variations of those same wrappers, constants reached through one level of
/// indirection, and shapes built to defeat the fail-closed rule itself.
///
/// A list of shapes is what a fixer special-cases, so the property that closes the family is
/// asserted separately by [`the_unknown_condition_fails_closed`], and two attacks no list
/// contains are covered by [`two_attacks_the_known_list_does_not_contain`].
#[test]
fn the_scrubber_actually_removes_every_decoy_shape() {
    use crate::v2::core::test_support::class_r_scrub::live_code;
    let cases = [
        // ── tier 1: the needle is text, not code
        ("line comment", "// set_loadout(x)\nlet a = 1;"),
        ("block comment", "/* set_loadout(x) */ let a = 1;"),
        ("nested block comment", "/* a /* set_loadout(x) */ b */ x"),
        ("string literal", "let s = \"set_loadout(x)\";"),
        ("raw string", "let s = r#\"set_loadout(x)\"#;"),
        // ── tier 2: the known dead-code wrappers
        ("if false", "if false { set_loadout(x); }"),
        ("if true == false", "if true == false { set_loadout(x); }"),
        ("if false == true", "if false == true { set_loadout(x); }"),
        ("if !true", "if !true { set_loadout(x); }"),
        ("if 1 > 2", "if 1 > 2 { set_loadout(x); }"),
        ("while false", "while false { set_loadout(x); }"),
        ("cfg(any())", "#[cfg(any())] fn d() { set_loadout(x); }"),
        (
            "cfg(any()) mod shadow copy",
            "#[cfg(any())] mod shadow { fn cargo_panel() { set_loadout(x); } }",
        ),
        ("after break", "loop { break; set_loadout(x); }"),
        ("after continue", "loop { continue; set_loadout(x); }"),
        ("after return", "fn f() { return; set_loadout(x); }"),
        (
            "match guard",
            "match () { _ if false => { set_loadout(x); } _ => {} }",
        ),
        (
            "const false binding",
            "const C: bool = false; fn f() { if C { set_loadout(x); } }",
        ),
        (
            "black_box(false)",
            "if std::hint::black_box(false) { set_loadout(x); }",
        ),
        ("cfg!(any())", "if cfg!(any()) { set_loadout(x); }"),
        // ── tier 3: spelling variations of the same wrappers, not new structures
        ("cfg(any()) spaced", "#[cfg( any() )] fn d() { set_loadout(x); }"),
        (
            "cfg(any()) spaced brackets",
            "#[ cfg(any()) ] fn d() { set_loadout(x); }",
        ),
        (
            "cfg(any()) inner spaces",
            "#[cfg(any( ))]\nfn d() { set_loadout(x); }",
        ),
        (
            "if condition with odd spacing",
            "if  true  ==  false  { set_loadout(x); }",
        ),
        (
            "black_box, core path",
            "if core::hint::black_box(1) > core::hint::black_box(2) { set_loadout(x); }",
        ),
        // ── measured against real files, not imagined. The first two passed the first cut of
        // this scrubber: after an earlier `const`/`let` failed its checks the binding scanner
        // resumed *inside* that binding's own text, from where it could never reach a later
        // one. Any guard whose file held such a binding above the decoy was hollow.
        (
            "const declared on the same line as the if",
            "fn f() {\nconst T601C: bool = false; if T601C {\n    set_loadout(x);\n}\n}",
        ),
        (
            "const folded through a comparison, same line",
            "fn f() {\nconst T601N: bool = 1 > 2; if T601N {\n    set_loadout(x);\n}\n}",
        ),
        (
            "const behind an unrelated non-bool const",
            "const OTHER: &str = \"x\";\nconst T601C: bool = false;\nfn f() { if T601C { set_loadout(x); } }",
        ),
        (
            "const behind a let-else",
            "fn g() { let Ok(v) = h() else { return; }; }\nconst T601C: bool = false;\nfn f() { if T601C { set_loadout(x); } }",
        ),
        // ── THE ONE THAT SHIPPED GREEN. `sse.rs`, `client.rs` and `arsenal.rs` all park their
        // live path inside a binding whose initializer is a block (`let run = async { … };`,
        // `let send = move |t| { … };`), and the binding scanner used to resume after the
        // initializer — so nothing inside one was ever seen. Measured against the real files.
        (
            "const nested inside a block-initialised binding",
            "fn f() { let run = async {\nconst T601C: bool = false; if T601C { set_loadout(x); }\n}; }",
        ),
        (
            "const nested inside a closure-initialised binding",
            "fn f() { let send = move |t| {\nconst T601N: bool = 1 > 2; if T601N { set_loadout(x); }\n}; }",
        ),
        (
            "const inside an async block",
            "fn f() { spawn(async move {\nconst T601C: bool = false; if T601C {\n    set_loadout(x);\n}\n}); }",
        ),
        // ── tier 4: six wrappers that survived an earlier evaluator, each measured on a real
        // production file before it was fixed. They are kept for regression value only; the
        // thing that closes the family is `the_unknown_condition_fails_closed`.
        (
            "const referencing const",
            "const W_A: bool = false; const W_B: bool = W_A;\nfn f() { if W_B { set_loadout(x); } }",
        ),
        (
            "the same chain, declared out of order",
            "const W_B: bool = W_A; const W_A: bool = false;\nfn f() { if W_B { set_loadout(x); } }",
        ),
        (
            "block-expression initialiser",
            "const W_NEVER: bool = { false };\nfn f() { if W_NEVER { set_loadout(x); } }",
        ),
        (
            "tuple index",
            "fn f() { if (true, false).1 { set_loadout(x); } }",
        ),
        (
            "arithmetic inside a comparison",
            "fn f() { if 1 + 1 > 3 { set_loadout(x); } }",
        ),
        (
            "bitwise rather than logical",
            "fn f() { if false | false { set_loadout(x); } }",
        ),
        (
            "leading :: on a transparent call",
            "fn f() { if ::std::hint::black_box(false) { set_loadout(x); } }",
        ),
        // ── tier 5: shapes invented against the fail-closed rule rather than handed down.
        // With the unknown case failing closed they cost nothing to defeat, which is the
        // point: none of them had to be thought of in advance.
        (
            "array index",
            "fn f() { if [false, true][0] { set_loadout(x); } }",
        ),
        (
            "if-expression const initialiser",
            "const W_C: bool = if true { false } else { true };\nfn f() { if W_C { set_loadout(x); } }",
        ),
        (
            "immediately-invoked closure",
            "fn f() { if (|| false)() { set_loadout(x); } }",
        ),
        (
            "xor",
            "fn f() { if false ^ false { set_loadout(x); } }",
        ),
        (
            "shift compared to a literal",
            "fn f() { if 1 << 2 == 7 { set_loadout(x); } }",
        ),
        (
            "constant laundered through a let",
            "fn f() { let w: bool = (true, false).1; if w { set_loadout(x); } }",
        ),
    ];
    for (label, src) in cases {
        let scrubbed = live_code(src);
        assert!(
            !scrubbed.contains("set_loadout"),
            "{label}: decoy survived scrubbing — every pin built on this scrubber is hollow \
             while staying green, which is the exact defect this scrubber exists to remove.\n{scrubbed}"
        );
    }

    // …and it must not eat live code while it is at it. A scrubber that removed everything
    // would pass every case above and pin nothing.
    let live = "if x { set_loadout(a); } else { set_loadout(b); }";
    assert_eq!(live_code(live).matches("set_loadout(").count(), 2);
    for kept in [
        "if 2 > 1 { set_loadout(a); }",
        "while running { set_loadout(a); }",
        "#[cfg(target_arch = \"wasm32\")] fn d() { set_loadout(a); }",
        "#[cfg(feature = \"never-enabled\")] fn d() { set_loadout(a); }",
        "const C: bool = true; fn f() { if C { set_loadout(a); } }",
        "match () { _ if x => { set_loadout(a); } _ => {} }",
        "fn f() { if a { return; } set_loadout(a); }",
        // The shapes a fail-closed evaluator could plausibly eat. Every one of these names
        // something the program computes, so none is constant-shaped and none may be scrubbed.
        // Without this half, "scrub whatever you cannot read" would pass the whole battery
        // above by deleting the crate.
        "fn f() { if let Some(v) = opt { set_loadout(v); } }",
        "fn f() { while let Some(v) = it.next() { set_loadout(v); } }",
        "fn f() { if resp.ok() { set_loadout(a); } }",
        "fn f() { let ok = resp.ok(); if ok { set_loadout(a); } }",
        "fn f() { let ok: bool = resp.ok(); if ok { set_loadout(a); } }",
        "fn f() { if !items.is_empty() { set_loadout(a); } }",
        "fn f() { if i < n { set_loadout(a); } }",
        "fn f() { if cfg!(feature = \"x\") { set_loadout(a); } }",
        "fn f() { if cfg!(target_arch = \"wasm32\") { set_loadout(a); } }",
        // A numeric `const` is compile-time material, so it MUST fold rather than fail closed —
        // otherwise every `const LIMIT: usize = …; if LIMIT > n` in the crate turns RED.
        "const LIMIT: usize = 5; fn f() { if LIMIT > 3 { set_loadout(a); } }",
        "const LIMIT: usize = 5; fn f() { if LIMIT > 3 && x { set_loadout(a); } }",
        "const NAME: &str = \"x\"; fn f() { if p == NAME { set_loadout(a); } }",
    ] {
        assert!(
            live_code(kept).contains("set_loadout"),
            "the scrubber ate live code: {kept}"
        );
    }
    // A lifetime is not a char literal; a `;` inside a type is not an item terminator.
    assert!(live_code("fn f<'a>(x: &'a str) { set_loadout(x); }").contains("'a"));
    assert!(
        live_code("#[cfg(any())] const D: [u8; 3] = [1, 2, 3];\nfn f() { set_loadout(x); }")
            .contains("set_loadout"),
        "the `;` inside `[u8; 3]` must not end the cfg'd item early"
    );
    // `live_source` keeps literals — a route path or a `data-testid` is shipped code.
    assert!(
        crate::v2::core::test_support::class_r_scrub::live_source("let p = \"/servers\";")
            .contains("/servers")
    );
    assert!(!live_code("let p = \"/servers\";").contains("/servers"));
}

/// The property behind the list: an unreadable condition fails closed.
///
/// Every earlier round of this defect was closed by enumerating the wrapper shapes that had
/// been reported, and each was walked around by the next spelling. Replacing the blocklists
/// with a real evaluator lost the same way, because every expression the evaluator could not
/// read fell through to "keep" — which means "report as live".
///
/// The battery above is regression value. This test asserts the invariant directly, over
/// conditions chosen so that no fix could have special-cased them, spelled with operators the
/// evaluator provably does not model.
///
/// The invariant has two halves and both are load-bearing:
///
/// 1. A condition naming nothing the program computes is a compile-time constant. If it does
///    not fold to `true`, the block goes — **whatever** shape it is.
/// 2. A condition naming anything the program computes is genuinely conditional and stays. A
///    "fail-closed" scrubber without this half would pass every attack test by deleting the
///    crate, and would turn every guard built on it permanently red.
#[test]
fn the_unknown_condition_fails_closed() {
    use crate::v2::core::test_support::class_r_scrub::live_code;

    // Half 1 — pure compile-time material, spelled with operators `lex` emits `Tok::Other`
    // for. None of these is parsed; all of them must still be removed.
    for cond in [
        "(true, false).1",
        "1 + 1 > 3",
        "false | false",
        "false ^ false",
        "[false, true][0]",
        "(|| false)()",
        "1 << 2 == 7",
        "10 % 3 == 2",
        "-1 > 0",
        "*&false",
        "(true && false) & true",
        "({ false })",
        "::std::hint::black_box(false)",
        "0xff_u8 as bool",
    ] {
        let src = format!("fn f() {{ if {cond} {{ set_loadout(x); }} }}");
        assert!(
            !live_code(&src).contains("set_loadout"),
            "`if {cond}` mentions nothing this program computes, so its truth was fixed at \
             compile time. The evaluator could not read it — and an evaluator that cannot \
             prove code is live must not report it as live. This is a false GREEN, the exact \
             defect five waves have now failed to close by enumeration."
        );
    }

    // The same shapes behind one level of `const` indirection, which is how the reproduction
    // on a real production file was built.
    for init in ["(true, false).1", "{ false }", "1 + 1 > 3", "false | false"] {
        let src = format!(
            "const W_A: bool = {init}; const W_B: bool = W_A;\n\
             fn f() {{ if W_B {{ set_loadout(x); }} }}"
        );
        assert!(
            !live_code(&src).contains("set_loadout"),
            "`const W_A: bool = {init}; const W_B: bool = W_A` — a `const` is compile-time by \
             Rust's own rules, so a `const` this pass cannot fold is a constant it failed to \
             read, never a runtime value"
        );
    }

    // Half 2 — one runtime name is enough to make the condition genuinely conditional. These
    // are the same operators; the only difference is that something in them is computed.
    for cond in [
        "(true, flag).1",
        "n + 1 > 3",
        "flag | false",
        "[flag, true][0]",
        "(|| flag)()",
        "resp.ok()",
        "!items.is_empty()",
        "cfg!(feature = \"x\")",
        "let Some(v) = opt",
    ] {
        let src = format!("fn f() {{ if {cond} {{ set_loadout(x); }} }}");
        assert!(
            live_code(&src).contains("set_loadout"),
            "`if {cond}` names something the program computes, so it is live code the \
             scrubber must leave alone. Eating it would turn every cure-2 pin permanently RED \
             — a fail-closed evaluator that scrubs the program is not a fix, it is an outage."
        );
    }

    // ── the residual, pinned so it cannot grow in silence ────────────────────────────────
    //
    // These DO survive, and the module doc says so. A call is the boundary: to this pass
    // `Option::<bool>::None.unwrap_or(false)` and `resp.ok()` are the same three tokens in the
    // same order, and there is no reading of the text that separates them. Folding calls by
    // name would be the blocklist again, one level down — and folding them *all* would delete
    // every `if resp.ok()` in the crate. So an opaque call stays live, loudly documented,
    // rather than quietly half-handled.
    //
    // Asserted rather than omitted: if a later change closes one of these, this test fails and
    // whoever closed it gets to move the line in the module doc too. That is the opposite of
    // how the last five rounds of this defect were "fixed".
    for cond in [
        "Option::<bool>::None.unwrap_or(false)",
        "bool::default()",
        "\"\".is_empty() && false == true",
    ] {
        let src = format!("fn f() {{ if {cond} {{ set_loadout(x); }} }}");
        assert!(
            live_code(&src).contains("set_loadout"),
            "`if {cond}` is a KNOWN residual (an opaque call). If it now scrubs, that is an \
             improvement — say so in the residual list at the top of this file instead of \
             leaving this assertion lying about what the scrubber does."
        );
    }
}

/// Two attacks no handed-down list contains.
///
/// The listed shapes are the ones a fix naturally special-cases, so passing them proves little
/// on its own. These two were built against the fix itself:
///
/// * **A1 — the shadow copy with no `cfg` at all.** The listed variant parks the decoy under
///   `#[cfg(any())]`, so every cfg-based defence catches it. Move the real item into a plain
///   `mod` nobody calls, leave the pristine copy at column 0, and there is no cfg to find, no
///   dead-code wrapper to strip, and both copies compile. Only refusing **ambiguity** catches
///   this, which is why `only_body` counts matches before it reads one.
/// * **A2 — the constant folded through a comparison.** The listed variant is
///   `const C: bool = false; if C`, which a fixer answers by looking for `= false`.
///   `const NEVER: bool = 1 > 2;` has no `false` anywhere in it. Only actually evaluating the
///   initialiser catches it.
///
/// Bonus third, same family as A2 but on the `cfg` side: `#[cfg(all(any(), unix))]` contains
/// `any` but is not the literal `#[cfg(any())]`, and `#[cfg(not(all()))]` contains neither.
#[test]
fn two_attacks_the_known_list_does_not_contain() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

    // A1 — pristine decoy at column 0, real (cut) code in a live module. No cfg, no wrapper.
    let a1 = "\
fn cargo_panel() { on_change(&items); }
mod real {
pub fn cargo_panel() { /* wire cut */ }
}
";
    let scrubbed = live_code(a1);
    let hits = scrubbed.matches("fn cargo_panel(").count();
    assert_eq!(
        hits, 2,
        "both definitions must survive scrubbing: {scrubbed}"
    );
    let caught = std::panic::catch_unwind(|| only_body(&scrubbed, "fn cargo_panel(")).is_err();
    assert!(
        caught,
        "A1: a shadow definition with no cfg and no dead-code wrapper fed the pin a decoy — \
         only an ambiguity refusal catches this shape"
    );

    // A2 — the constant never spells `false`.
    let a2 = "const NEVER: bool = 1 > 2;\nfn f() { if NEVER { on_change(&items); } }";
    assert!(
        !live_code(a2).contains("on_change"),
        "A2: `const NEVER: bool = 1 > 2` must fold — a fixer that grepped for `= false` \
         would have shipped this hole"
    );

    // Bonus — composite never-true cfg predicates.
    for src in [
        "#[cfg(all(any(), unix))] fn d() { on_change(&items); }",
        "#[cfg(not(all()))] fn d() { on_change(&items); }",
        "#[cfg(any(any(), any()))] fn d() { on_change(&items); }",
    ] {
        assert!(
            !live_code(src).contains("on_change"),
            "composite false cfg survived: {src}"
        );
    }
}
