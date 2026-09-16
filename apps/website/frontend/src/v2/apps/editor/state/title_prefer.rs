//! Role: the pins that hold the mission row's metadata wire together across the engine wall.
//! Position: `editor/state` in the frontend.
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
mod t505_tests {
    use website_map_engine::editing::persist::server_adoption::{
        payload_title_nonblank, prefer_payload_title,
    };

    /// The prefer helper must keep the authored title when the row is stale.
    ///
    /// RED: make `prefer_payload_title` always return `row_title.trim()`, or drop the prefer.
    #[test]
    fn prefer_payload_keeps_authored_over_stale_row() {
        let payload = r#"{"title":"  Authored Bridgehead  ","editor":{}}"#;
        assert_eq!(
            prefer_payload_title(payload, "Stale Library Title"),
            "Authored Bridgehead"
        );
        assert_eq!(
            prefer_payload_title(r#"{"title":"   "}"#, "  Row Title  "),
            "Row Title"
        );
        assert_eq!(
            prefer_payload_title(r#"{"editor":{}}"#, "Row Only"),
            "Row Only"
        );
    }

    #[test]
    fn payload_title_nonblank_trim() {
        assert_eq!(
            payload_title_nonblank(r#"{"title":"  Authored  "}"#).as_deref(),
            Some("Authored")
        );
        assert_eq!(payload_title_nonblank(r#"{"title":"  "}"#), None);
        assert_eq!(payload_title_nonblank(r#"{"editor":{}}"#), None);
    }

    /// The adopt must route the title through the prefer helper rather than writing the row's.
    ///
    /// RED: pass `&row.title` straight into `apply_row_meta`, or drop `prefer_payload_title` /
    /// `payload_title_nonblank` from the adopt body.
    ///
    /// Superseded in strength by `t570_tests`, which observes the title `apply_row_meta` actually
    /// received: this one still greps, so it is kept only as a fast, readable first failure.
    #[test]
    fn adopt_payload_wires_prefer_helper() {
        const SRC: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/persist/server_adoption.rs"
        ));
        let production = SRC.split("#[cfg(test)]").next().unwrap_or(SRC);
        let adopt = production
            .split("fn adopt_payload(")
            .nth(1)
            .and_then(|s| s.split("\n}\n").next())
            .expect("adopt_payload body");
        assert!(
            adopt.contains("prefer_payload_title(")
                || adopt.contains("payload_title_nonblank(payload_json)"),
            "adopt_payload must prefer via prefer_payload_title / payload_title_nonblank; got:\n{adopt}"
        );
        assert!(
            !adopt.contains("&row.title,"),
            "adopt_payload must not pass &row.title straight into apply_row_meta (stomp); got:\n{adopt}"
        );
    }
}

/// The row-meta wire, pinned **behaviourally**, across both sides of the engine wall.
///
/// The wire is made of six items in two crates: the engine's `struct RowMeta`, `impl RowMeta`,
/// `fn non_empty`, `adopt_payload` and `apply_row_meta_only`, plus the frontend's
/// `row_meta_from_detail`, which is the one place the API's row shape is read. This module lifts all
/// six **verbatim**, compiles them against a recording stand-in for the document, runs the result,
/// and asserts on the arguments `apply_row_meta` was handed on the path that actually executed.
///
/// The question "is this call site reachable?" is undecidable from source text, so the pin stops
/// asking it and *executes the text* instead. An unreachable call contributes nothing to the
/// recording no matter how it was made unreachable.
///
/// ## What a GREEN here does and does not claim
///
/// * **Does:** starting from a row, the source as committed runs `row_meta_from_detail` →
///   `adopt_payload` / `apply_row_meta_only` → exactly one `apply_row_meta` call each, and that call
///   carries the row's briefing / time / weather through the real `non_empty`, plus (adopt) a title
///   routed through `prefer_payload_title`.
/// * **Does not:** say anything about the document's own `apply_row_meta` — argument *positions* are
///   fixed by the mock's signature, so a parameter reorder inside the store is that module's own
///   pin to catch. Nor does it cover the fetch that builds the row, which stays unpinned.
/// * **Known residual:** the harness reads its evidence from the generated program's stdout, so
///   production source that printed this module's sentinels could forge a record. That is the
///   irreducible limit of running code you are also judging — it is sabotage, not a wrapper, and the
///   sentinels are unique strings that do not otherwise occur in the tree.
///
/// ## Deliberate fail-closed edges
///
/// * No compiler → RED. The pin never skips; a tool that cannot examine its input must not pass.
/// * A pinned item that grows a new dependency stops compiling here → RED until
///   [`HARNESS_PREAMBLE`] is extended. Loud and cheap; the alternative is a pin that quietly stops
///   covering the thing it names.
/// * The harness runs **natively** while half the wire ships on **wasm32**, so conditional
///   compilation inside a pinned item is the one construct that could make the two disagree. Rather
///   than mis-decide it, [`item_in`] rejects any `cfg` on or inside a pinned item outright.
#[cfg(test)]
mod t570_tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// The engine's half of the wire — the adoption policy — verbatim, at compile time.
    const ADOPTION_SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/editing/persist/server_adoption.rs"
    ));
    /// The frontend's half — the one place the API's row shape is read.
    const HYDRATE_SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/state/hydrate.rs"
    ));

    const ADOPT_SIG: &str = "fn adopt_payload(";
    const APPLY_SIG: &str = "fn apply_row_meta_only(";
    const OPT_SIG: &str = "fn non_empty(";
    const ROW_STRUCT_SIG: &str = "struct RowMeta {";
    const ROW_IMPL_SIG: &str = "impl RowMeta {";
    /// Pinned as well, so the harness drives the whole wire rather than only the engine's end of it.
    /// Cutting the briefing one hop earlier — `briefing: String::new()` where the row is read — is
    /// the same user-visible bug, and no grep generation ever looked there.
    const ROW_FROM_SIG: &str = "fn row_meta_from_detail(";

    /// Field separator in the harness's stdout (cannot occur in Rust source or in our sentinels).
    const US: char = '\u{1f}';

    const ROW_TITLE: &str = "T570-ROW-TITLE";
    const TERRAIN: &str = "T570-TERRAIN";
    const TOD: &str = "T570-TIME-OF-DAY";
    const WEATHER: &str = "T570-WEATHER";
    const ADOPT_BRIEFING: &str = "T570-ADOPT-BRIEFING";
    const APPLY_BRIEFING: &str = "T570-APPLY-BRIEFING";
    /// What the harness's `prefer_payload_title` stub returns. Seeing it in the title position is
    /// proof the *executed* adopt path routed the title through the prefer helper (T-505/T-522) —
    /// a stale `&row.title` would show up as [`ROW_TITLE`] instead.
    const PREFERRED: &str = "T570-PREFERRED-TITLE";
    const END: &str = "T570-HARNESS-END";

    /// Mock doc core + the crate items the pinned bodies reach for. Everything here is scaffolding;
    /// the only code under test is the verbatim text spliced in after it.
    const HARNESS_PREAMBLE: &str = r#"// GENERATED by website-frontend `title_prefer::t570_tests`.
// The items below the preamble are copied VERBATIM out of the sources they pin — do not edit.
#![allow(dead_code, unused_variables, unused_mut, unreachable_code, clippy::all)]

use std::cell::RefCell;

const DEFAULT_LAYER_ID: &str = "T570-LAYER";
const PREFER_SENTINEL: &str = "@PREFERRED@";

/// The `GET /missions/:id` row, cut down to the five fields `row_meta_from_detail` reads. The rest
/// of the real row type is irrelevant to this wire; if the reader starts reading a sixth field the
/// harness stops compiling, which is the correct answer to "the row wire changed".
struct MissionDetail {
    title: String,
    terrain: String,
    time_of_day: String,
    weather: String,
    briefing: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Adopt {
    Init,
    Undoable,
}

/// Recording stand-in for `MissionDocCore`: it reports what the pinned source ACTUALLY passed,
/// on the path that actually ran.
#[derive(Default)]
struct Core {
    calls: RefCell<Vec<String>>,
}

impl Core {
    fn set_origin_init(&self, _init: bool) {}

    fn hydrate(&self, _payload_json: &str, _layer_id: &str) {}

    fn apply_row_meta(
        &self,
        title: &str,
        terrain: &str,
        time_of_day: Option<String>,
        weather: Option<String>,
        briefing: Option<String>,
    ) {
        self.calls.borrow_mut().push(format!(
            "{}\u{1f}{}\u{1f}{:?}\u{1f}{:?}\u{1f}{:?}",
            title, terrain, time_of_day, weather, briefing
        ));
    }
}

type DocHandle = RefCell<Option<Core>>;

/// Stand-in for the adoption module's own prefer helper. Seeing its sentinel in the title position
/// is proof the executed adopt routed the title through it.
fn prefer_payload_title(_payload_json: &str, _row_title: &str) -> String {
    PREFER_SENTINEL.to_string()
}

fn report(label: &str, doc: &DocHandle) {
    if let Some(core) = doc.borrow().as_ref() {
        for call in core.calls.borrow().iter() {
            println!("{}\u{1f}{}", label, call);
        }
    }
}

/// The row exactly as the app gets it, through the real `row_meta_from_detail`.
fn row(briefing: &str) -> RowMeta {
    row_meta_from_detail(&MissionDetail {
        title: "@ROW_TITLE@".to_string(),
        terrain: "@TERRAIN@".to_string(),
        time_of_day: "@TOD@".to_string(),
        weather: "@WEATHER@".to_string(),
        briefing: Some(briefing.to_string()),
    })
}

fn drive(
    label: &str,
    adopt: fn(&DocHandle, &str, &RowMeta, Adopt, &dyn Fn()),
    apply: fn(&DocHandle, &RowMeta),
) {
    let doc: DocHandle = RefCell::new(Some(Core::default()));
    adopt(
        &doc,
        "{\"title\":\"T570-PAYLOAD-TITLE\",\"editor\":{}}",
        &row("@ADOPT_BRIEFING@"),
        Adopt::Init,
        &|| {},
    );
    report(&format!("{}-adopt", label), &doc);

    let doc: DocHandle = RefCell::new(Some(Core::default()));
    apply(&doc, &row("@APPLY_BRIEFING@"));
    report(&format!("{}-apply", label), &doc);
}

"#;

    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn find_from(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
        if needle.is_empty() || hay.len() < needle.len() {
            return None;
        }
        (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
    }

    /// Any whole-word identifier in the `cfg` family — `cfg`, `cfg_attr`, `cfg_match`, whatever
    /// the next one is called.
    ///
    /// The prefix rule is deliberate. T-601's first cut asked for the whole word `cfg` and let
    /// `#[cfg_attr(target_arch = "wasm32", …)]` through, which is the same class of miss as the
    /// hole it was written to close: conditional compilation spelled slightly differently. A pin
    /// that runs code natively to prove a wasm wire is live must refuse *every* construct that can
    /// make the two builds disagree, not the one spelling someone thought of.
    fn mentions_cfg_family(hay: &[char]) -> bool {
        let mut i = 0;
        while i < hay.len() {
            if is_ident_char(hay[i]) && (i == 0 || !is_ident_char(hay[i - 1])) {
                let s = i;
                while i < hay.len() && is_ident_char(hay[i]) {
                    i += 1;
                }
                let word: String = hay[s..i].iter().collect();
                if word == "cfg" || word.starts_with("cfg_") {
                    return true;
                }
                continue;
            }
            i += 1;
        }
        false
    }

    /// Same-length copy of `chars` with comments and string literals blanked to spaces (newlines
    /// kept, so line numbers survive).
    ///
    /// Indices into the mask are indices into the original, which is the point: the extractor can
    /// find a signature and balance braces without a `{` in a comment or a `"…fn adopt_payload(…"`
    /// in a literal steering it. Char literals are not lexed (no pinned item contains one; a stray
    /// `'{'` would unbalance extraction into a compile error — RED, never a silent pass), and
    /// nested block comments are not either.
    fn masked(chars: &[char]) -> Vec<char> {
        fn blank(c: char) -> char {
            if c == '\n' {
                c
            } else {
                ' '
            }
        }
        let mut out: Vec<char> = Vec::with_capacity(chars.len());
        let mut i = 0;
        while i < chars.len() {
            // `// …` to end of line
            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                while i < chars.len() && chars[i] != '\n' {
                    out.push(blank(chars[i]));
                    i += 1;
                }
                continue;
            }
            // `/* … */`
            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                out.push(' ');
                out.push(' ');
                i += 2;
                while i < chars.len()
                    && !(chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/')
                {
                    out.push(blank(chars[i]));
                    i += 1;
                }
                let tail = (chars.len() - i).min(2);
                for k in 0..tail {
                    out.push(blank(chars[i + k]));
                }
                i += tail;
                continue;
            }
            // `r"…"` / `r#"…"#`
            if chars[i] == 'r' && (i == 0 || !is_ident_char(chars[i - 1])) {
                let mut j = i + 1;
                let mut hashes = 0usize;
                while j < chars.len() && chars[j] == '#' {
                    hashes += 1;
                    j += 1;
                }
                if j < chars.len() && chars[j] == '"' {
                    for c in &chars[i..=j] {
                        out.push(blank(*c));
                    }
                    i = j + 1;
                    while i < chars.len() {
                        if chars[i] == '"' {
                            let mut k = 0usize;
                            while k < hashes && i + 1 + k < chars.len() && chars[i + 1 + k] == '#' {
                                k += 1;
                            }
                            if k == hashes {
                                let end = (i + 1 + hashes).min(chars.len());
                                for c in &chars[i..end] {
                                    out.push(blank(*c));
                                }
                                i = end;
                                break;
                            }
                        }
                        out.push(blank(chars[i]));
                        i += 1;
                    }
                    continue;
                }
            }
            // `"…"` with `\` escapes
            if chars[i] == '"' {
                out.push(' ');
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        out.push(' ');
                        out.push(blank(chars[i + 1]));
                        i += 2;
                        continue;
                    }
                    let quote = chars[i] == '"';
                    out.push(blank(chars[i]));
                    i += 1;
                    if quote {
                        break;
                    }
                }
                continue;
            }
            out.push(chars[i]);
            i += 1;
        }
        assert_eq!(
            out.len(),
            chars.len(),
            "T-570: mask lost alignment with the source — extraction cannot be trusted"
        );
        out
    }

    /// Start of the item **including everything that annotates it**: its `#[…]` / `#![…]`
    /// attributes and the modifiers that can sit between them and the signature (`pub`,
    /// `pub(crate)`, `async`, `unsafe`, `const`, `extern`, `default`).
    ///
    /// # Why this exists (T-601, closing the wave-77 F2 hole)
    ///
    /// [`item`]'s `cfg` refusal used to scan only the extracted span, `start..=end` — the signature
    /// and the body. An attribute on the line **above** the signature is outside that span, and so
    /// was invisible to it. Demonstrated attack: move the real `adopt_payload` into a `mod` with a
    /// `pub use` and cut its briefing, then leave a pristine copy at column 0 under
    /// `#[cfg(any())]`. The wasm build ships the cut wire; this harness compiles and runs the
    /// decoy, sees the briefing it wanted, and reports ok. Two independent defects in one move —
    /// the ambiguity count missed the indented real item (fixed below), and the `cfg` refusal
    /// missed the attribute above the decoy (fixed here).
    ///
    /// Comments are already blanked to spaces by [`masked`], so walking backwards over whitespace
    /// steps over doc comments for free. The walk is deliberately **greedy**: over-including a
    /// preceding token can only make the `cfg` refusal stricter (a false RED, which is loud),
    /// while under-including is how the hole above stayed silent.
    fn attr_start(mask: &[char], sig_at: usize) -> usize {
        const MODIFIERS: &[&str] = &[
            "pub", "async", "unsafe", "const", "extern", "default", "static", "move",
        ];
        let mut a = sig_at;
        loop {
            let mut b = a;
            while b > 0 && mask[b - 1].is_whitespace() {
                b -= 1;
            }
            if b == 0 {
                return 0;
            }
            match mask[b - 1] {
                // `#[ … ]` / `#![ … ]`
                ']' => {
                    let mut depth = 0usize;
                    let mut k = b - 1;
                    let open = loop {
                        match mask[k] {
                            ']' => depth += 1,
                            '[' => {
                                depth -= 1;
                                if depth == 0 {
                                    break Some(k);
                                }
                            }
                            _ => {}
                        }
                        if k == 0 {
                            break None;
                        }
                        k -= 1;
                    };
                    let Some(open) = open else { return b };
                    let mut h = open;
                    if h > 0 && mask[h - 1] == '!' {
                        h -= 1;
                    }
                    if h > 0 && mask[h - 1] == '#' {
                        a = h - 1;
                        continue;
                    }
                    return b;
                }
                // `pub(crate)` / `pub(super)` / `pub(in path)`
                ')' => {
                    let mut depth = 0usize;
                    let mut k = b - 1;
                    let open = loop {
                        match mask[k] {
                            ')' => depth += 1,
                            '(' => {
                                depth -= 1;
                                if depth == 0 {
                                    break Some(k);
                                }
                            }
                            _ => {}
                        }
                        if k == 0 {
                            break None;
                        }
                        k -= 1;
                    };
                    match open {
                        Some(open) => {
                            a = open;
                            continue;
                        }
                        None => return b,
                    }
                }
                c if is_ident_char(c) => {
                    let mut k = b;
                    while k > 0 && is_ident_char(mask[k - 1]) {
                        k -= 1;
                    }
                    let word: String = mask[k..b].iter().collect();
                    if MODIFIERS.contains(&word.as_str()) {
                        a = k;
                        continue;
                    }
                    return b;
                }
                _ => return b,
            }
        }
    }

    /// Verbatim source of the one `sig` item in the engine's adoption module.
    fn item(sig: &str) -> String {
        item_in(ADOPTION_SRC, sig)
    }

    /// [`item`], with the source as a parameter so the extractor itself can be pinned against
    /// synthetic attacks — see [`the_extractor_refuses_the_shapes_that_beat_it`]. An extractor that
    /// silently stopped refusing decoys would leave this whole module green over a dead wire, which
    /// is the defect it exists to remove; it does not get to be the one untested thing here.
    fn item_in(src: &str, sig: &str) -> String {
        let chars: Vec<char> = src.chars().collect();
        let mask = masked(&chars);
        let needle: Vec<char> = sig.chars().collect();

        // EVERY occurrence, at any indentation. A second definition of the same item — the obvious
        // way to feed the pin a pristine copy while the real one is cut — is ambiguity, and
        // ambiguity is RED.
        //
        // T-601: this used to count only **column-0** occurrences, which is precisely half the
        // W77-F2 attack: move the real item into a `mod` (indented, therefore uncounted) and leave
        // a decoy at column 0, and `heads.len()` reads 1 while the pin examines the wrong copy.
        // Comments and string literals are already blanked by `masked`, so a surviving occurrence
        // of `fn adopt_payload(` / `struct RowMeta {` in the mask is a definition, full stop —
        // there is nothing left for the column-0 filter to protect against, only something for it
        // to hide.
        let mut heads = Vec::new();
        let mut from = 0;
        while let Some(i) = find_from(&mask, &needle, from) {
            heads.push(i);
            from = i + 1;
        }
        assert_eq!(
            heads.len(),
            1,
            "T-570: expected exactly one `{sig}` in the pinned source, found {}. \
             0 means it was renamed or deleted; 2+ means a shadow definition — and a nested `mod` \
             copy compiles perfectly well beside the real one, so 'it would not build' is not a \
             defence. The pin cannot examine code it cannot unambiguously find, so this is RED, \
             not a skip.",
            heads.len()
        );
        let start = heads[0];

        let mut i = start;
        while i < mask.len() && mask[i] != '{' {
            i += 1;
        }
        assert!(i < mask.len(), "T-570: `{sig}` has no body");
        let mut depth = 0usize;
        let end = loop {
            match mask[i] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break i;
                    }
                }
                _ => {}
            }
            i += 1;
            assert!(i < mask.len(), "T-570: `{sig}` body never closes");
        };

        // The scan starts at the item's ATTRIBUTES, not at its signature — see [`attr_start`].
        let guarded = attr_start(&mask, start);
        assert!(
            !mentions_cfg_family(&mask[guarded..=end]),
            "T-570: `{sig}` is under, or contains, conditional compilation.\n\
             This pin executes the item natively to prove the wire is live, so a `cfg` anywhere on \
             it would let the wasm build and this harness disagree — which is exactly the hole the \
             W67 `#[cfg(any())]` decoy walked through, and (T-601) the hole a `#[cfg(any())]` on \
             the line ABOVE the signature walked through after that. Conditional compilation \
             belongs outside the pinned items."
        );

        chars[start..=end].iter().collect()
    }

    /// Rename a pinned item so several variants of it can live in one generated program.
    fn renamed(item: &str, sig: &str, name: &str) -> String {
        assert!(
            item.starts_with(sig),
            "T-570: `{sig}` extraction misaligned"
        );
        format!("fn {name}({}", &item[sig.len()..])
    }

    /// `(label, adopt_item, apply_item)` triples → one compilable program.
    fn harness_source(variants: &[(&str, String, String)]) -> String {
        let mut src = HARNESS_PREAMBLE
            .replace("@PREFERRED@", PREFERRED)
            .replace("@ROW_TITLE@", ROW_TITLE)
            .replace("@TERRAIN@", TERRAIN)
            .replace("@TOD@", TOD)
            .replace("@WEATHER@", WEATHER)
            .replace("@ADOPT_BRIEFING@", ADOPT_BRIEFING)
            .replace("@APPLY_BRIEFING@", APPLY_BRIEFING);

        for sig in [ROW_STRUCT_SIG, ROW_IMPL_SIG, OPT_SIG] {
            src.push_str(&item(sig));
            src.push_str("\n\n");
        }
        src.push_str(&item_in(HYDRATE_SRC, ROW_FROM_SIG));
        src.push_str("\n\n");
        let mut main = String::from("fn main() {\n");
        for (label, adopt, apply) in variants {
            src.push_str(&renamed(adopt, ADOPT_SIG, &format!("adopt_{label}")));
            src.push_str("\n\n");
            src.push_str(&renamed(apply, APPLY_SIG, &format!("apply_{label}")));
            src.push_str("\n\n");
            main.push_str(&format!(
                "    drive(\"{label}\", adopt_{label}, apply_{label});\n"
            ));
        }
        main.push_str(&format!("    println!(\"{END}\");\n}}\n"));
        src.push_str(&main);
        src
    }

    /// The compiler that built this test. Cargo exports `CARGO` to every crate it compiles and
    /// `rustc` is its sibling in the same toolchain; PATH is the fallback. There is deliberately no
    /// "skip if absent" branch — a pin that cannot examine its input must go RED.
    fn rustc_bin() -> PathBuf {
        if let Some(cargo) = option_env!("CARGO") {
            let sibling = Path::new(cargo).with_file_name("rustc");
            if sibling.is_file() {
                return sibling;
            }
        }
        PathBuf::from("rustc")
    }

    /// Compile + run the generated program; stdout lines, newest evidence first-hand.
    ///
    /// The scratch directory is left behind on failure so the exact source that failed can be read.
    fn run(tag: &str, variants: &[(&str, String, String)]) -> Vec<String> {
        let source = harness_source(variants);
        let dir = std::env::current_exe()
            .expect("T-570: test executable path")
            .with_file_name(format!("t570-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("T-570: cannot create {}: {e}", dir.display()));
        let rs = dir.join("t570_pin.rs");
        std::fs::write(&rs, &source)
            .unwrap_or_else(|e| panic!("T-570: cannot write {}: {e}", rs.display()));
        let bin = dir.join("t570_pin");

        let rustc = rustc_bin();
        let compiled = Command::new(&rustc)
            .args(["--edition", "2021", "--crate-name", "t570_pin"])
            .args(["-C", "debug-assertions=on", "-A", "warnings", "-o"])
            .arg(&bin)
            .arg(&rs)
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "T-570: cannot run `{}`: {e}\n\
                     This pin proves the hydrate wire by compiling and running it, so a missing \
                     compiler is a failure to verify, not a pass.",
                    rustc.display()
                )
            });
        assert!(
            compiled.status.success(),
            "T-570: the extracted hydrate items no longer compile against the pin's mock core.\n\
             Usually this means a pinned item grew a dependency the harness preamble does not \
             model — extend HARNESS_PREAMBLE. Source kept at {}\n\n{}",
            rs.display(),
            String::from_utf8_lossy(&compiled.stderr)
        );

        let ran = Command::new(&bin)
            .output()
            .unwrap_or_else(|e| panic!("T-570: cannot run {}: {e}", bin.display()));
        assert!(
            ran.status.success(),
            "T-570: the extracted hydrate path aborted. Source kept at {}\n\nstdout:\n{}\nstderr:\n{}",
            rs.display(),
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        );
        let stdout = String::from_utf8_lossy(&ran.stdout).into_owned();
        assert_eq!(
            stdout.lines().last(),
            Some(END),
            "T-570: the harness did not run to completion; stdout was:\n{stdout}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        stdout.lines().map(str::to_string).collect()
    }

    /// The raw evidence, with the unit separator swapped for something a human can read.
    fn evidence(lines: &[String]) -> String {
        lines.join("\n").replace(US, " | ")
    }

    /// Recorded `apply_row_meta` argument tuples for one `label-adopt` / `label-apply` channel.
    fn observed(lines: &[String], channel: &str) -> Vec<String> {
        let prefix = format!("{channel}{US}");
        lines
            .iter()
            .filter_map(|l| l.strip_prefix(&prefix).map(str::to_string))
            .collect()
    }

    fn live_items() -> (String, String) {
        (item(ADOPT_SIG), item(APPLY_SIG))
    }

    /// T-554 … T-570 Class-R: hydrate must hand the row's briefing to `apply_row_meta` on the path
    /// that actually runs.
    ///
    /// RED: replace either `non_empty(&row.briefing)` with `None`.
    /// RED (W63–W67 decoys): comment / block-comment / string / dead `let _` / `if false` /
    /// `if true == false` / `loop { break; … }` / `#[cfg(any())]` / `while false` / `if !true` —
    /// and every wrapper nobody has invented yet. An unreachable call is not executed, so it
    /// records nothing, so it cannot green this.
    /// RED (T-570 additions): neuter `fn opt` to return `None`; hide the call behind
    /// `if std::hint::black_box(false)` or a `const FALSE: bool` the optimiser folds; `return`
    /// above it; delete the call outright.
    #[test]
    fn hydrate_wires_row_briefing_into_apply_row_meta() {
        let (adopt, apply) = live_items();
        let lines = run("pin", &[("live", adopt, apply)]);

        let want_adopt =
            format!("{PREFERRED}{US}{TERRAIN}{US}Some({TOD:?}){US}Some({WEATHER:?}){US}Some({ADOPT_BRIEFING:?})");
        let want_apply =
            format!("{ROW_TITLE}{US}{TERRAIN}{US}Some({TOD:?}){US}Some({WEATHER:?}){US}Some({APPLY_BRIEFING:?})");

        for (channel, want, fname) in [
            ("live-adopt", &want_adopt, "adopt_payload"),
            ("live-apply", &want_apply, "apply_row_meta_only"),
        ] {
            let got = observed(&lines, channel);
            assert_eq!(
                got.len(),
                1,
                "T-570: `{fname}` performed {} apply_row_meta calls with a non-empty row; the row \
                 meta is written exactly once. 0 means the live call never ran — a dead-code \
                 wrapper or a deleted wire; this pin executes the source, so unreachable code is \
                 invisible to it by design.\nrecorded:\n{}",
                got.len(),
                evidence(&lines)
            );
            assert_eq!(
                &got[0],
                want,
                "T-570: `{fname}` passed the wrong row meta to apply_row_meta.\n\
                 fields are title | terrain | time_of_day | weather | briefing.\n\
                 A `None` briefing means the wire is cut ({fname} must pass non_empty(&row.briefing)); \
                 a briefing of Some(\"\") means `fn non_empty` no longer maps the row string; a title of \
                 {ROW_TITLE:?} in the adopt channel means the stale row title stomped the payload \
                 title (T-505).\nrecorded:\n{}",
                evidence(&lines)
            );
        }

        // Two records + the end marker, and nothing else: an unexpected line means something in
        // the pinned source is writing to the channel this pin reads its evidence from.
        assert_eq!(
            lines.len(),
            3,
            "T-570: unexpected harness output:\n{}",
            evidence(&lines)
        );
    }

    /// **T-601 — the extractor's own pin: the two shapes that beat it in wave 77.**
    ///
    /// [`hydrate_wires_row_briefing_into_apply_row_meta`] is sound *given* that [`item`] hands it
    /// the code that actually ships. Wave 77's F2 finding was that it did not have to: the
    /// ambiguity check counted only column-0 signature heads, and the `cfg` refusal scanned only
    /// inside the extracted span. Move the real item into a `mod` (indented → uncounted) and leave
    /// a pristine copy at column 0 under `#[cfg(any())]` (attribute above the signature →
    /// unscanned) and the harness compiles and runs the copy the wasm build never sees.
    ///
    /// Both halves are asserted here, plus the shapes a fixer might have papered over instead of
    /// fixing. Each must panic; a returned `String` means the hole is back.
    #[test]
    fn the_extractor_refuses_the_shapes_that_beat_it() {
        let refuses = |label: &str, src: &str| {
            let owned = src.to_string();
            let r = std::panic::catch_unwind(move || item_in(&owned, "fn adopt_payload("));
            assert!(
                r.is_err(),
                "T-601: `{label}` was accepted by the extractor. This pin executes what `item` \
                 returns, so accepting the wrong copy is a GREEN over a dead wire — the exact \
                 defect this module exists to remove."
            );
        };

        // ── the full W77-F2 attack: pristine column-0 decoy + real code moved into a `mod`.
        refuses(
            "column-0 pristine decoy with the real code in a mod",
            "\
#[cfg(any())]
fn adopt_payload(doc: &D, p: &str, row: &R, mode: A) {
    core.apply_row_meta(&t, &row.terrain, non_empty(&row.time_of_day), non_empty(&row.weather), non_empty(&row.briefing));
}
mod real {
    pub fn adopt_payload(doc: &D, p: &str, row: &R, mode: A) {
        core.apply_row_meta(&t, &row.terrain, non_empty(&row.time_of_day), non_empty(&row.weather), None);
    }
}
pub use real::adopt_payload;
",
        );

        // ── half one on its own: a shadow copy anywhere, with no cfg at all to give it away.
        refuses(
            "shadow copy in a plain mod, no cfg anywhere",
            "\
fn adopt_payload(doc: &D) { live(); }
mod shadow {
    pub fn adopt_payload(doc: &D) { dead(); }
}
",
        );
        refuses(
            "shadow copy indented inside an impl",
            "fn adopt_payload(a: u8) { x(); }\nimpl T {\n    fn adopt_payload(a: u8) { y(); }\n}\n",
        );

        // ── half two on its own: a cfg attribute on the line ABOVE the signature.
        for (label, attr) in [
            ("cfg(any()) above the signature", "#[cfg(any())]"),
            ("spaced cfg above the signature", "#[cfg( any() )]"),
            (
                "target_arch above the signature",
                "#[cfg(target_arch = \"wasm32\")]",
            ),
            (
                "cfg_attr above the signature",
                "#[cfg_attr(test, allow(dead_code))]",
            ),
        ] {
            refuses(
                label,
                &format!("{attr}\nfn adopt_payload(a: u8) {{ live(); }}\n"),
            );
            // …and still refused when a doc comment sits between the attribute and the signature,
            // since `masked` blanks the comment and the walk-back must step over it.
            refuses(
                &format!("{label}, doc comment between"),
                &format!("{attr}\n/// what it does\nfn adopt_payload(a: u8) {{ live(); }}\n"),
            );
            // …and when the item carries a visibility modifier, which is where a naive
            // walk-back would stop short and miss the attribute entirely.
            refuses(
                &format!("{label}, pub(crate) item"),
                &format!("{attr}\npub(crate) fn adopt_payload(a: u8) {{ live(); }}\n"),
            );
        }

        // The instrument must still say YES to the honest shape, or every assertion above is
        // satisfied by an extractor that refuses everything and pins nothing.
        let ok = item_in(
            "/// doc\n#[inline]\npub fn adopt_payload(a: u8) { live(); }\n",
            "fn adopt_payload(",
        );
        assert_eq!(ok, "fn adopt_payload(a: u8) { live(); }");
        // A decoy inside a comment or a string is still not a definition.
        let ok = item_in(
            "// fn adopt_payload(x) {}\nlet s = \"fn adopt_payload(y) {}\";\nfn adopt_payload(a: u8) { live(); }\n",
            "fn adopt_payload(",
        );
        assert_eq!(ok, "fn adopt_payload(a: u8) { live(); }");
    }

    /// Calibration — proof this instrument can still say NO, and proof of the specific thing five
    /// grep generations got wrong.
    ///
    /// Takes the same live items and deliberately breaks them two ways, then asserts the harness
    /// reports the break: `dead` wraps each body in `if true == false { … }` (the W67 attack) and
    /// must record nothing at all; `none` cuts the briefing argument and must record `None`. If
    /// this test ever passes vacuously, the pin above is decoration.
    #[test]
    fn dead_and_cut_wires_are_visible_to_this_pin() {
        let (adopt, apply) = live_items();
        let dead = |src: &str| {
            let open = src.find('{').expect("body");
            format!(
                "{}{{ if true == false {{{}}} }}",
                &src[..open],
                &src[open + 1..src.len() - 1]
            )
        };
        let cut = |src: &str| src.replace("non_empty(&row.briefing)", "None");
        let lines = run(
            "calibration",
            &[
                ("dead", dead(&adopt), dead(&apply)),
                ("none", cut(&adopt), cut(&apply)),
            ],
        );

        for channel in ["dead-adopt", "dead-apply"] {
            assert!(
                observed(&lines, channel).is_empty(),
                "T-570: an `if true == false {{ … }}` body still recorded a call on {channel} — \
                 the harness is not observing execution.\nrecorded:\n{}",
                evidence(&lines)
            );
        }
        for channel in ["none-adopt", "none-apply"] {
            let got = observed(&lines, channel);
            assert_eq!(got.len(), 1, "T-570: {channel} lost its call");
            assert!(
                got[0].ends_with("None"),
                "T-570: a cut briefing argument still read back as {:?} on {channel} — the pin is \
                 not reading the argument it claims to.",
                got[0]
            );
        }
    }
}
