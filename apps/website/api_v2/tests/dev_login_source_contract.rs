//! Source-level contract for the dev-login handler: every role must bind its own discord id
//! and arma id through the `discord_id_for_role` / `arma_id_for_role` helpers.
//!
//! The scanners below read the handler as text, so they need no database and fail fast. They
//! are a first failure, not the contract — `tests/dev_login_runtime_identity.rs` reads the
//! identities back over HTTP, which is the only view that can see whether the arms compile.

/// Strip `//` and `/* */` outside string/char/raw-string literals so a source pin cannot stay green
/// when live match arms are moved into comments.
///
/// Local copy — `common::strip_rust_comments_outside_literals` is private to that module; do
/// not widen it.
///
/// Lifetimes (`'static`) must not open a char-string span: that leaves `//` arms inside
/// `fn … -> &'static str` bodies un-stripped, which is a hollow green.
fn strip_rust_comments_outside_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        // Raw / byte / plain strings — copy whole literal (no comment strip inside).
        if let Some((_cs, _ce, full_end)) = string_literal_span(bytes, i) {
            out.push_str(&src[i..full_end]);
            i = full_end;
            continue;
        }
        // Lifetimes (`'static`, `'a`) — emit `'` only; do not enter char-lit mode.
        if bytes[i] == b'\'' {
            let next = bytes.get(i + 1).copied();
            if next.is_some_and(|c| c.is_ascii_alphabetic() || c == b'_') {
                out.push('\'');
                i += 1;
                continue;
            }
            // Real char literal — copy through.
            out.push('\'');
            i += 1;
            while i < bytes.len() {
                let c = bytes[i];
                out.push(c as char);
                if c == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                i += 1;
                if c == b'\'' {
                    break;
                }
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Span of a Rust string/raw-string/byte-string starting at `i`:
/// `(content_start, content_end, full_end)` — content is exclusive of delimiters.
///
/// Needed so `r#" "enlisted" => … "#` decoys are recognised as one literal — naive `"`
/// scanning splits them and leaves the arm text searchable.
fn string_literal_span(bytes: &[u8], i: usize) -> Option<(usize, usize, usize)> {
    let n = bytes.len();
    if i > 0 {
        let prev = bytes[i - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            // Mid-identifier (`foo_r`, `sr"…"`) — not a string prefix at `i`.
            return None;
        }
    }
    let mut p = i;
    // Optional `b` / `c` prefix before `r` or `"`.
    if p < n && matches!(bytes[p], b'b' | b'c' | b'B' | b'C') {
        if p + 1 < n && matches!(bytes[p + 1], b'r' | b'R' | b'"') {
            p += 1;
        } else {
            return None;
        }
    }
    let mut raw = false;
    if p < n && matches!(bytes[p], b'r' | b'R') {
        raw = true;
        p += 1;
    }
    let mut hashes = 0usize;
    if raw {
        while p < n && bytes[p] == b'#' {
            hashes += 1;
            p += 1;
        }
    }
    if p >= n || bytes[p] != b'"' {
        return None;
    }
    let content_start = p + 1;
    if raw {
        // Closer is `"` + the same number of `#`.
        let mut q = content_start;
        while q < n {
            if bytes[q] == b'"' {
                let mut h = 0usize;
                while q + 1 + h < n && bytes[q + 1 + h] == b'#' {
                    h += 1;
                }
                if h == hashes {
                    return Some((content_start, q, q + 1 + hashes));
                }
            }
            q += 1;
        }
        None
    } else {
        let mut q = content_start;
        while q < n {
            if bytes[q] == b'\\' && q + 1 < n {
                q += 2;
                continue;
            }
            if bytes[q] == b'"' {
                return Some((content_start, q, q + 1));
            }
            q += 1;
        }
        None
    }
}

/// Blank the interiors of string / raw-string literals **except** when the literal is a
/// match-arm pattern (followed by `=>`).
///
/// Comment strip alone keeps a source pin green when live arms collapse to `_` and the
/// old arms live only inside `r#" "enlisted" => … "#` decoys (string contents survive
/// comment strip). Blanking non-pattern string interiors removes that hollow while
/// keeping real `"enlisted" => DEV_…` arms searchable.
fn blank_string_contents_except_match_patterns(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some((content_start, content_end, full_end)) = string_literal_span(bytes, i) {
            let mut j = full_end;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let is_match_pattern = j + 1 < bytes.len() && bytes[j] == b'=' && bytes[j + 1] == b'>';
            if is_match_pattern {
                out.push_str(&src[i..full_end]);
            } else {
                out.push_str(&src[i..content_start]);
                for _ in content_start..content_end {
                    out.push(' ');
                }
                out.push_str(&src[content_end..full_end]);
            }
            i = full_end;
            continue;
        }
        // Char literals — copy through (match arms use `"…"` patterns).
        // Lifetimes (`'static`, `'a`) look like a leading `'` + ident; do NOT swallow them
        // as char lits (that would skip the rest of the fn and leave decoys unblanked).
        if bytes[i] == b'\'' {
            let next = bytes.get(i + 1).copied();
            if next.is_some_and(|c| c.is_ascii_alphabetic() || c == b'_') {
                out.push('\'');
                i += 1;
                continue;
            }
            out.push('\'');
            i += 1;
            while i < bytes.len() {
                let c = bytes[i];
                out.push(c as char);
                if c == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                i += 1;
                if c == b'\'' {
                    break;
                }
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Fn-body view for the match-arm source pins: comment-stripped, then non-pattern string
/// interiors blanked, so a raw-string decoy cannot green.
fn live_match_view(fn_body: &str) -> String {
    blank_string_contents_except_match_patterns(fn_body)
}

/// Brace-balanced body of the **file-scope** item `fn {name}` in comment-stripped source
/// (opening `{` … matching `}`).
///
/// # Why this counts braces instead of calling `str::find`
///
/// A *nested* `mod decoy { async fn dev_login(…) { …a faithful copy… } }` placed above the real
/// item wins `str::find`, so every assertion below would read the decoy's body and report
/// success over source it never examined. With such a decoy carrying dev_login's helper binds
/// and its INSERT, `dev_login_roles_use_distinct_discord_ids` stays GREEN while the live
/// handler has lost its COALESCE first-create.
///
/// The fix is not a ban on `mod`: this function always meant the file-scope item, so it counts
/// braces (skipping literals, which [`string_literal_span`] already knows how to span) and
/// takes only markers at depth 0. Requiring exactly one costs nothing — two file-scope
/// `fn dev_login`s do not compile — and turns "which one did it read?" into a named failure.
fn fn_body<'a>(code: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}");
    let m = marker.as_bytes();
    let all = code.as_bytes();
    let mut depth = 0i32;
    let mut k = 0usize;
    let mut starts: Vec<usize> = Vec::new();
    while k < all.len() {
        if let Some((_cs, _ce, full_end)) = string_literal_span(all, k) {
            k = full_end;
            continue;
        }
        if all[k] == b'\'' {
            // Lifetime vs char literal — needed here so a `'static`
            // return type cannot desynchronise the brace count for the rest of the file.
            let next = all.get(k + 1).copied();
            if next.is_some_and(|c| c.is_ascii_alphabetic() || c == b'_') {
                k += 1;
                continue;
            }
            k += 1;
            while k < all.len() {
                let c = all[k];
                if c == b'\\' && k + 1 < all.len() {
                    k += 2;
                    continue;
                }
                k += 1;
                if c == b'\'' {
                    break;
                }
            }
            continue;
        }
        match all[k] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {
                if depth == 0
                    && all[k..].starts_with(m)
                    && (k == 0 || !(all[k - 1].is_ascii_alphanumeric() || all[k - 1] == b'_'))
                    && all
                        .get(k + m.len())
                        .is_none_or(|c| !(c.is_ascii_alphanumeric() || *c == b'_'))
                {
                    starts.push(k);
                }
            }
        }
        k += 1;
    }
    let start = match starts.as_slice() {
        [only] => *only,
        [] => panic!(
            "source pin: no FILE-SCOPE `{marker}` in comment-stripped \
             src/identity_and_access/handlers/developer_login.rs — a definition nested in a `mod`/`impl`/block is not \
             the item the crate calls, and this pin binds the top-level one on purpose."
        ),
        many => panic!(
            "source pin: {} file-scope `{marker}` definitions (offsets {many:?}) — a Rust \
             file cannot have two, so this source is not what the crate compiles.",
            many.len()
        ),
    };
    let after = &code[start..];
    let open = after
        .find('{')
        .unwrap_or_else(|| panic!("source pin: `{marker}` has no opening brace"));
    let bytes = after.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        // Skip whole string / raw-string literals (braces inside r#"…"#).
        if let Some((_cs, _ce, full_end)) = string_literal_span(bytes, i) {
            i = full_end;
            continue;
        }
        if bytes[i] == b'\'' {
            let next = bytes.get(i + 1).copied();
            if next.is_some_and(|c| c.is_ascii_alphabetic() || c == b'_') {
                i += 1; // lifetime
                continue;
            }
            // Char literal.
            i += 1;
            while i < bytes.len() {
                let c = bytes[i];
                if c == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                i += 1;
                if c == b'\'' {
                    break;
                }
            }
            continue;
        }
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return &after[open..=i];
            }
        }
        i += 1;
    }
    panic!("source pin: `{marker}` body not closed");
}

/// Each role must map to a distinct discord_id (and arma_id) in the dev-login handler.
///
/// Perturbation RED:
/// - delete a role-specific literal, OR
/// - collapse `discord_id_for_role` / `arma_id_for_role` so a role arm no longer binds its
///   dedicated constant (dead literals alone must not keep this green — measured hollow), OR
/// - move live match arms into `//` / `/* */` comments (a raw-source grep stays green), OR
/// - bind `discord_id = DEV_USER_ID` / `arma_id = DEV_ARMA_ID` at the call site while the
///   helpers remain as dead code, OR
/// - park arms only inside `r#" "enlisted" => … "#` (or plain string) decoys while the live
///   match collapses to `_` (comment strip alone keeps string contents).
///
/// A single `…001` / single arma literal is the fold every one of those produces.
///
/// # The documented limit of this pin
///
/// **This test cannot decide whether the arms it finds are compiled.** `#[cfg(any())]` on the
/// three role arms with a live `_ => DEV_USER_ID` underneath walks straight around it: the arms
/// are still in the file, so the comment-strip + string-blank view still sees them and this
/// stays GREEN while every role folds back onto one row.
///
/// Teaching the view about `#[cfg(any())]` is **not** the answer, for one reason:
/// reachability is not a lexical property. `if false`, `const NEVER: bool = false`,
/// `std::hint::black_box(false)`, an early `return`, a macro that expands to something else,
/// and a `use other as …` shadow all reach the same result with no attribute to find. The
/// frontend's `arsenal.rs` does carry that machinery — a `cfg`-predicate parser that
/// constant-folds the condition and treats one it *cannot* fold as dead rather than live, so an
/// unrecognised wrapper costs a loud false RED instead of a silent false GREEN. That is worth a
/// whole module in a crate with no runtime signature to reach for. It is worth nothing here,
/// where the invariant *does* have one and
/// `dev_login_gives_every_role_its_own_identity_at_runtime` already reads it back over HTTP.
///
/// **Shapes this pin still admits (GREEN while the behaviour is gone):** any never-true `cfg` on
/// the arms or on either helper item; any constant-false guard around the `match`; a `return`
/// above it; macro-generated or `include!`d bodies. It is kept because it is fast, needs no
/// database, and names the exact literal that drifted — it is a first failure, not the contract.
///
/// **The contract is `dev_login_gives_every_role_its_own_identity_at_runtime`** in
/// `tests/dev_login_runtime_identity.rs`, which logs in as all four roles and reads the
/// identities back over HTTP.
#[test]
fn dev_login_roles_use_distinct_discord_ids() {
    let handler = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/identity_and_access/handlers/developer_login.rs");
    let src = std::fs::read_to_string(&handler)
        .unwrap_or_else(|e| panic!("source pin: read {}: {e}", handler.display()));
    // The pins below run on comment-stripped code so `// "enlisted" => …` cannot green.
    let code = strip_rust_comments_outside_literals(&src);

    let discord_ids = [
        "000000000000000001", // admin / default
        "000000000000000002", // enlisted
        "000000000000000003", // leader
        "000000000000000004", // mission_maker
    ];
    let arma_ids = [
        "dev-arma-76561190000000001",
        "dev-arma-76561190000000002",
        "dev-arma-76561190000000003",
        "dev-arma-76561190000000004",
    ];
    for id in discord_ids {
        assert!(
            code.contains(id),
            "src/identity_and_access/handlers/developer_login.rs missing discord_id `{id}` — each role needs its own \
             row"
        );
    }
    for id in arma_ids {
        assert!(
            code.contains(id),
            "src/identity_and_access/handlers/developer_login.rs missing arma_id `{id}` — per-role COALESCE must not race \
             idx_users_arma_id"
        );
    }
    assert!(
        code.contains("fn discord_id_for_role"),
        "expected discord_id_for_role helper — literals alone are not the contract"
    );
    assert!(
        code.contains("fn arma_id_for_role"),
        "expected arma_id_for_role helper — literals alone are not the contract"
    );

    // Live arms inside the helpers — comment-stripped, then non-pattern string interiors
    // blanked, so raw-string decoys cannot green.
    let discord_live = live_match_view(fn_body(&code, "discord_id_for_role"));
    let arma_live = live_match_view(fn_body(&code, "arma_id_for_role"));
    for arm in [
        "\"enlisted\" => DEV_USER_ID_ENLISTED",
        "\"leader\" => DEV_USER_ID_LEADER",
        "\"mission_maker\" => DEV_USER_ID_MISSION_MAKER",
    ] {
        assert!(
            discord_live.contains(arm),
            "expected live role arm `{arm}` inside discord_id_for_role \
             (comment-stripped + non-pattern strings blanked) — commented-out arms or \
             r#\" … \"# decoys reintroduce the single-id fold"
        );
    }
    for arm in [
        "\"enlisted\" => DEV_ARMA_ID_ENLISTED",
        "\"leader\" => DEV_ARMA_ID_LEADER",
        "\"mission_maker\" => DEV_ARMA_ID_MISSION_MAKER",
    ] {
        assert!(
            arma_live.contains(arm),
            "expected live role arm `{arm}` inside arma_id_for_role \
             (comment-stripped + non-pattern strings blanked) — commented-out arms or \
             r#\" … \"# decoys reintroduce the single-id fold"
        );
    }

    // Call site must *use* the helpers: dead helpers plus a DEV_USER_ID bind
    // must go red. Exact `let … = …(role);` — not the `fn …(role: &str)` signature.
    let login_body = fn_body(&code, "dev_login");
    assert!(
        login_body.contains("let discord_id = discord_id_for_role(role);"),
        "dev_login must bind `let discord_id = discord_id_for_role(role);` — \
         `discord_id = DEV_USER_ID` with helpers left dead reintroduces the single-id fold"
    );
    assert!(
        login_body.contains("let arma_id = arma_id_for_role(role);"),
        "dev_login must bind `let arma_id = arma_id_for_role(role);` — \
         `arma_id = DEV_ARMA_ID` with helpers left dead races idx_users_arma_id again"
    );

    // Keep the shape: NULL insert + live COALESCE (do not re-introduce an INSERT stamp).
    // Soft pin here; common/mod.rs owns the sqlx::query-payload COALESCE gate.
    assert!(
        code.contains("SET arma_id = COALESCE(arma_id"),
        "must keep the live COALESCE first-create path"
    );
    assert!(
        !src.contains("'', 'dev-arma-76561190000000001'"),
        "must not reintroduce a fixed arma_id as an INSERT VALUES literal"
    );
}

/// Raw-string (and plain-string) arm decoys must not satisfy the match-arm pin;
/// live `"role" => CONST` arms must still be visible.
#[test]
fn raw_string_arm_decoy_is_blanked_live_arms_kept() {
    // Build the hollow shape via concat! (nested raw strings break a surrounding r#").
    let decoy = concat!(
        "fn discord_id_for_role(role: &str) -> &'static str {\n",
        "    let _decoy = r#\"\n",
        "        \"enlisted\" => DEV_USER_ID_ENLISTED,\n",
        "        \"leader\" => DEV_USER_ID_LEADER,\n",
        "        \"mission_maker\" => DEV_USER_ID_MISSION_MAKER,\n",
        "    \"#;\n",
        "    match role {\n",
        "        _ => DEV_USER_ID,\n",
        "    }\n",
        "}\n",
    );
    let live = concat!(
        "fn discord_id_for_role(role: &str) -> &'static str {\n",
        "    match role {\n",
        "        \"enlisted\" => DEV_USER_ID_ENLISTED,\n",
        "        \"leader\" => DEV_USER_ID_LEADER,\n",
        "        \"mission_maker\" => DEV_USER_ID_MISSION_MAKER,\n",
        "        _ => DEV_USER_ID,\n",
        "    }\n",
        "}\n",
    );
    let arms = [
        "\"enlisted\" => DEV_USER_ID_ENLISTED",
        "\"leader\" => DEV_USER_ID_LEADER",
        "\"mission_maker\" => DEV_USER_ID_MISSION_MAKER",
    ];

    // Sanity: a comment-strip-only view stays hollow-green on the decoy.
    for arm in arms {
        assert!(
            decoy.contains(arm),
            "fixture sanity: decoy must embed `{arm}` before blanking"
        );
    }

    let decoy_view = live_match_view(decoy);
    let live_view = live_match_view(live);
    for arm in arms {
        assert!(
            !decoy_view.contains(arm),
            "raw-string arm decoy must go RED after blanking; still found `{arm}` in:\n{decoy_view}"
        );
        assert!(
            live_view.contains(arm),
            "live match arm `{arm}` must stay visible (do not false-red valid binds)"
        );
    }

    // Comment-arm attack still RED on the live-match view after comment strip.
    let commented = concat!(
        "fn discord_id_for_role(role: &str) -> &'static str {\n",
        "    match role {\n",
        "        // \"enlisted\" => DEV_USER_ID_ENLISTED,\n",
        "        /* \"leader\" => DEV_USER_ID_LEADER, */\n",
        "        _ => DEV_USER_ID,\n",
        "    }\n",
        "}\n",
    );
    let commented_view = live_match_view(&strip_rust_comments_outside_literals(commented));
    assert!(
        !commented_view.contains("\"enlisted\" => DEV_USER_ID_ENLISTED"),
        "// comment-arm must stay RED"
    );
    assert!(
        !commented_view.contains("\"leader\" => DEV_USER_ID_LEADER"),
        "/* */ comment-arm must stay RED"
    );
}
