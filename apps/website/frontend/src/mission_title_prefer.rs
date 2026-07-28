//! T-522 / T-505 — prefer non-blank payload title over a stale missions-row title.
//! T-554 / T-559 / T-561 / T-564 / T-567 — native Class-R ratchet for the FE hydrate→
//! `apply_row_meta` briefing wire (`opt(&row.briefing)` in `adopt_payload` / `apply_row`).
//!
//! Pure helper extracted from `mission_hydrate` so Class-R runs on native
//! `cargo test -p website-frontend` (cold gate). The live hydrate glue stays
//! `#[cfg(target_arch = "wasm32")]`; without this module a prefer→`&row.title`
//! regression stayed green on CI. The briefing pin lives here for the same reason
//! (W62: both sites → `None` left website-frontend green while only core Class-R
//! covered `apply_row_meta` itself).
//!
//! T-559: the T-554 soft `body.contains("opt(&row.briefing)")` stayed green when both
//! live wires became `None` but a `// … opt(&row.briefing)` decoy remained. Strip Rust
//! line comments before the contains so only a live call-site can satisfy the pin.
//!
//! T-561: `//`-only strip still left `/* opt(&row.briefing) */` and
//! `let _ = "opt(&row.briefing)";` green. Strip block comments + string literals too.
//!
//! T-564: strip still left `let _ = opt(&row.briefing);` + `None` args green. Require the
//! needle inside a live `apply_row_meta(…)` argument list, not anywhere in the fn body.
//!
//! T-567: arg-list scan still left `if false { apply_row_meta(…, opt(&row.briefing)) }` +
//! live `None` green (unreachable call still matched). Drop `if false { … }` blocks before
//! collecting arg lists so only a reachable call can satisfy the pin.

/// Non-blank trimmed top-level `title` from a compiled payload (T-375 wire emit).
///
/// Prefer this over the mission-row title when adopting: hydrate loads it into meta, but
/// a subsequent `apply_row_meta` with a stale row would otherwise stomp it. Whitespace-only is not a
/// title (same spirit as `eden_chrome` / `compile_payload`).
pub(crate) fn payload_title_nonblank(payload_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(payload_json).ok()?;
    v.get("title")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Prefer-payload rule `adopt_payload` must use: non-blank payload title, else row title.
pub(crate) fn prefer_payload_title(payload_json: &str, row_title: &str) -> String {
    payload_title_nonblank(payload_json).unwrap_or_else(|| row_title.trim().to_string())
}

#[cfg(test)]
mod t505_tests {
    use super::{payload_title_nonblank, prefer_payload_title};

    /// T-505 Class-R: prefer helper must keep authored title when the row is stale.
    ///
    /// RED: change `prefer_payload_title` to always return `row_title.trim()` (or drop prefer).
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

    /// T-505 Class-R: `adopt_payload` in mission_hydrate.rs must call the prefer helper.
    ///
    /// RED: pass `&row.title` straight into `apply_row_meta` (or drop `prefer_payload_title` /
    /// `payload_title_nonblank` from the adopt body).
    #[test]
    fn adopt_payload_wires_prefer_helper() {
        const SRC: &str = include_str!("mission_hydrate.rs");
        let production = SRC.split("#[cfg(test)]").next().unwrap_or(SRC);
        let adopt = production
            .split("fn adopt_payload(")
            .nth(1)
            .and_then(|s| s.split("\nfn ").next())
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

#[cfg(test)]
mod t554_tests {
    /// Production body of `fn name(` … next top-level `fn ` (T-522 ratchet shape).
    fn fn_body<'a>(production: &'a str, sig: &str) -> &'a str {
        production
            .split(sig)
            .nth(1)
            .and_then(|s| s.split("\nfn ").next())
            .unwrap_or_else(|| panic!("{sig} body"))
    }

    /// Drop Rust `//` / `///` / `/* … */` comments and `"…"` / `r#…#` string literals so a
    /// decoy substring cannot false-green the briefing wire pin (T-559 / T-561).
    ///
    /// Character scan (not line-naive): `//` inside a string must not truncate the literal,
    /// and a block-comment / string decoy must not survive into the pin body.
    fn strip_rust_comments_and_strings(src: &str) -> String {
        let chars: Vec<char> = src.chars().collect();
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < chars.len() {
            // Line comment `//…` → EOL
            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            // Block comment `/* … */`
            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i = (i + 2).min(chars.len());
                continue;
            }
            // Raw string `r##"…"##` / `r"…"`
            if chars[i] == 'r' {
                let mut j = i + 1;
                let mut hashes = 0usize;
                while j < chars.len() && chars[j] == '#' {
                    hashes += 1;
                    j += 1;
                }
                if j < chars.len() && chars[j] == '"' {
                    i = j + 1;
                    loop {
                        if i >= chars.len() {
                            break;
                        }
                        if chars[i] == '"' {
                            let mut k = 0usize;
                            while k < hashes && i + 1 + k < chars.len() && chars[i + 1 + k] == '#' {
                                k += 1;
                            }
                            if k == hashes {
                                i += 1 + hashes;
                                break;
                            }
                        }
                        i += 1;
                    }
                    continue;
                }
            }
            // Normal string `"…"` (with `\` escapes)
            if chars[i] == '"' {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\\' {
                        i = (i + 2).min(chars.len());
                        continue;
                    }
                    if chars[i] == '"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    /// Drop reachable-dead `if false { … }` blocks (brace-balanced) so an unreachable
    /// `apply_row_meta(…, opt(&row.briefing))` decoy cannot false-green the pin (T-567).
    ///
    /// Runs after comment/string strip. Leaves a trailing `else { … }` intact when present
    /// so a live else-arm call still counts. Exact shape: `\bif\b` + ws + `false` + ws* + `{`.
    fn strip_unreachable_if_false(src: &str) -> String {
        let chars: Vec<char> = src.chars().collect();
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < chars.len() {
            let at_if = chars[i] == 'i'
                && i + 1 < chars.len()
                && chars[i + 1] == 'f'
                && (i == 0 || !ident_char(chars[i - 1]))
                && (i + 2 >= chars.len() || !ident_char(chars[i + 2]));
            if at_if {
                let mut j = i + 2;
                while j < chars.len() && chars[j].is_ascii_whitespace() {
                    j += 1;
                }
                let is_false = j + 5 <= chars.len()
                    && chars[j] == 'f'
                    && chars[j + 1] == 'a'
                    && chars[j + 2] == 'l'
                    && chars[j + 3] == 's'
                    && chars[j + 4] == 'e'
                    && (j + 5 >= chars.len() || !ident_char(chars[j + 5]));
                if is_false {
                    let mut k = j + 5;
                    while k < chars.len() && chars[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if k < chars.len() && chars[k] == '{' {
                        // Skip `if false { … }` (balanced); leave any following `else`.
                        let mut depth = 1usize;
                        k += 1;
                        while k < chars.len() && depth > 0 {
                            match chars[k] {
                                '{' => depth += 1,
                                '}' => depth -= 1,
                                _ => {}
                            }
                            k += 1;
                        }
                        i = k;
                        continue;
                    }
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    fn ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// Collect paren-balanced argument lists of every `apply_row_meta(` call in `body`.
    ///
    /// Used so a dead `let _ = opt(&row.briefing);` cannot false-green the pin (T-564): the
    /// needle must appear *inside* an `apply_row_meta(…)` argument list. Callers must pass a
    /// body already scrubbed of comments/strings **and** `if false { … }` (T-567).
    fn apply_row_meta_arg_lists(body: &str) -> Vec<&str> {
        const CALL: &str = "apply_row_meta(";
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(rel) = body[from..].find(CALL) {
            let args_start = from + rel + CALL.len();
            let bytes = body.as_bytes();
            let mut depth = 1usize;
            let mut i = args_start;
            while i < body.len() && depth > 0 {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            if depth == 0 {
                out.push(&body[args_start..i - 1]);
            }
            from = i;
        }
        out
    }

    fn live_apply_row_meta_arg_contains(body: &str, needle: &str) -> bool {
        apply_row_meta_arg_lists(body)
            .into_iter()
            .any(|args| args.contains(needle))
    }

    /// Scrub comments/strings + unreachable `if false` arms, then require the needle inside a
    /// remaining `apply_row_meta(…)` argument list.
    fn scrubbed_live_wire(body: &str, needle: &str) -> bool {
        let scrubbed = strip_unreachable_if_false(&strip_rust_comments_and_strings(body));
        live_apply_row_meta_arg_contains(&scrubbed, needle)
    }

    /// T-554 / T-559 / T-561 / T-564 / T-567 Class-R: FE hydrate must pass a *reachable*
    /// `opt(&row.briefing)` as an `apply_row_meta` argument (comments + string literals
    /// stripped; dead `let _ = …` and `if false { apply_row_meta(…) }` are not enough).
    ///
    /// RED (W62): replace both `opt(&row.briefing)` with `None`.
    /// RED (W63 decoy): both → `None` + leave `// … opt(&row.briefing)` in each fn body.
    /// RED (W64 decoy): both → `None` + `/* opt(&row.briefing) */` or `let _ = "…";`.
    /// RED (W65 decoy): both → `None` + dead `let _ = opt(&row.briefing);`.
    /// RED (W66 decoy): both → `None` + `if false { apply_row_meta(…, opt(&row.briefing)) }`.
    #[test]
    fn hydrate_wires_row_briefing_into_apply_row_meta() {
        const SRC: &str = include_str!("mission_hydrate.rs");
        let production = SRC.split("#[cfg(test)]").next().unwrap_or(SRC);
        let adopt = fn_body(production, "fn adopt_payload(");
        let apply = fn_body(production, "fn apply_row(");
        const NEEDLE: &str = "opt(&row.briefing)";
        assert!(
            scrubbed_live_wire(adopt, NEEDLE),
            "adopt_payload must pass live opt(&row.briefing) as an apply_row_meta argument; got:\n{}",
            strip_unreachable_if_false(&strip_rust_comments_and_strings(adopt))
        );
        assert!(
            scrubbed_live_wire(apply, NEEDLE),
            "apply_row must pass live opt(&row.briefing) as an apply_row_meta argument; got:\n{}",
            strip_unreachable_if_false(&strip_rust_comments_and_strings(apply))
        );
    }
}
