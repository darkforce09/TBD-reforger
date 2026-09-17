//! Role: the pins that hold the mission row's metadata wire together across the engine wall.
//! Position: `editor/shell` in the frontend.
//! Signals & state: none; every item here is a test.
//! Invariants: the wire runs `GET /missions/:id` row -> `row_meta_from_detail` -> `RowMeta` ->
//! the adopt -> `apply_row_meta`, and it crosses a crate boundary in the middle. The
//! frontend half is compiled for the browser only, so no native test can link and call it; the
//! engine half can be called directly but proving the two halves still MEET takes evidence from
//! both files at once. That is what this module is for, and it is why it lives on the frontend side
//! of the wall rather than beside either half.
//!
//! # The invariant being held
//!
//! An adopt must carry the mission ROW's library blurb into the document — the blurb is a row
//! field, and if the wire is cut the editor silently shows an empty briefing and the next save
//! writes that emptiness back. It must also prefer the payload's own title over the row's, or a
//! stale row stomps the authored name on every boot.
//!
//! # Why the pin runs the code instead of reading it
//!
//! Five generations of source-scanning pins shipped hollow, each beaten by the next verifier:
//!
//! | scanned for | walked around by |
//! |-------------|------------------|
//! | `body.contains("…(&row.briefing)")` | a `//` comment decoy |
//! | the same, `//` stripped | `/* … */` and `let _ = "…(&row.briefing)";` |
//! | the same, block comments and strings stripped | a dead `let _ = …(&row.briefing);` |
//! | the needle inside an `apply_row_meta(…)` arg list | `if false { apply_row_meta(…) }` |
//! | the same, exact `if false { … }` blocks dropped | `if true == false` / `loop { break; … }` /
//!   `#[cfg(any())]` / `while false` / `if !true` |
//!
//! The last fix was a wrapper blocklist, and a blocklist can always be walked around: deciding
//! whether a call site is *reachable* from its source text is the halting problem in a costume
//! (`if 1 > 2`, `const C: bool = false; if C`, `if std::hint::black_box(false)`, a `return` above
//! it, a feature flag nobody enables …). A sixth grep generation would be the same bug.
//!
//! So [`t570_tests`] changes the **instrument**: it lifts the real items out of both files, compiles
//! them against a recording stand-in for the document, **runs** them, and asserts on the arguments
//! `apply_row_meta` actually received. Dead code produces no behaviour, so every wrapper — the five
//! above and every one nobody has invented yet — fails by construction rather than by enumeration.

#[cfg(test)]
#[path = "tests/title_prefer/payload_title_preference.rs"]
mod t505_tests;

#[cfg(test)]
#[path = "tests/title_prefer/row_metadata_wire.rs"]
mod t570_tests;
