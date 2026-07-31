//! Dev-login redirect/default-role + request-id + CORS middleware. Ports the Go
//! `dev_login_test.go` and the request-id / CORS cases of `middleware_test.go`.
//! Plus (T-235) the `servers` admin CRUD lifecycle — see §T-235 at the bottom.
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

const ORIGIN: &str = "http://localhost:5173";

async fn boot() -> Option<Router> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(app::router(AppState::new(
        pool,
        Config::for_tests(url, "misc-secret"),
    )))
}

#[tokio::test]
async fn dev_login_redirects_to_spa() {
    let Some(app) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    assert!(loc.starts_with(ORIGIN), "redirects to SPA: {loc}");
    assert!(loc.contains("access_token="), "carries the token fragment");
}

#[tokio::test]
async fn dev_login_unknown_role_defaults_to_admin() {
    let Some(app) = boot().await else { return };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=wizard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap();
    // The minted identity is an admin → /me reports role admin.
    let me = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["user"]["role"], "admin");
}

/// T-566 / T-387: strip `//` and `/* */` outside string/char/raw-string literals so Class-R
/// cannot stay green when live match arms are moved into comments (W65 hollow).
///
/// Local copy — `common::strip_rust_comments_outside_literals` is private (T-560/T-562 owns
/// that module; do not widen).
///
/// T-569: lifetimes (`'static`) must not open a char-string span — that left `//` arms
/// inside `fn … -> &'static str` bodies un-stripped (hollow green).
fn t566_strip_rust_comments_outside_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        // Raw / byte / plain strings — copy whole literal (no comment strip inside).
        if let Some((_cs, _ce, full_end)) = t569_string_literal_span(bytes, i) {
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
/// T-569: needed so `r#" "enlisted" => … "#` decoys are recognized as one literal
/// (naive `"` scanning splits them and leaves arm text searchable).
fn t569_string_literal_span(bytes: &[u8], i: usize) -> Option<(usize, usize, usize)> {
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

/// T-569 / W66: blank interiors of string / raw-string literals **except** when the
/// literal is a match-arm pattern (followed by `=>`).
///
/// Comment strip alone keeps Class-R green when live arms collapse to `_` and the
/// old arms live only inside `r#" "enlisted" => … "#` decoys (string contents survive
/// comment strip). Blanking non-pattern string interiors removes that hollow while
/// keeping real `"enlisted" => DEV_…` arms searchable.
fn t569_blank_string_contents_except_match_patterns(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some((content_start, content_end, full_end)) = t569_string_literal_span(bytes, i) {
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

/// Fn-body view for match-arm Class-R: comment-stripped, then non-pattern string
/// interiors blanked (T-569 raw-string decoy).
fn t569_live_match_view(fn_body: &str) -> String {
    t569_blank_string_contents_except_match_patterns(fn_body)
}

/// Brace-balanced body of the **file-scope** item `fn {name}` in comment-stripped source
/// (opening `{` … matching `}`).
///
/// # T-572 — `str::find` returned the wrong item
///
/// T-571 measured this on `common/mod.rs`'s twin of this function and it is true here too: a
/// *nested* `mod decoy { async fn dev_login(…) { …a faithful copy… } }` placed above the real
/// item wins `str::find`, so every assertion below read the decoy's body and reported success
/// over source it never examined. Measured on this tree with A1 applied: with the decoy carrying
/// dev_login's helper binds and its INSERT, `t387_dev_login_roles_use_distinct_discord_ids`
/// stayed GREEN while the live handler had lost its COALESCE first-create.
///
/// The fix is not a ban on `mod`: this function always meant the file-scope item, so it counts
/// braces (skipping literals, which [`t569_string_literal_span`] already knows how to span) and
/// takes only markers at depth 0. Requiring exactly one costs nothing — two file-scope
/// `fn dev_login`s do not compile — and turns "which one did it read?" into a named failure.
fn t566_fn_body<'a>(code: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}");
    let m = marker.as_bytes();
    let all = code.as_bytes();
    let mut depth = 0i32;
    let mut k = 0usize;
    let mut starts: Vec<usize> = Vec::new();
    while k < all.len() {
        if let Some((_cs, _ce, full_end)) = t569_string_literal_span(all, k) {
            k = full_end;
            continue;
        }
        if all[k] == b'\'' {
            // Lifetime vs char literal — the T-569 distinction, needed here so a `'static`
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
            "T-566/T-572 Class-R: no FILE-SCOPE `{marker}` in comment-stripped \
             src/handlers/dev.rs — a definition nested in a `mod`/`impl`/block is not the item \
             the crate calls, and this pin binds the top-level one on purpose."
        ),
        many => panic!(
            "T-572 Class-R: {} file-scope `{marker}` definitions (offsets {many:?}) — a Rust \
             file cannot have two, so this source is not what the crate compiles.",
            many.len()
        ),
    };
    let after = &code[start..];
    let open = after
        .find('{')
        .unwrap_or_else(|| panic!("T-566 Class-R: `{marker}` has no opening brace"));
    let bytes = after.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        // Skip whole string / raw-string literals (T-569: braces inside r#"…"#).
        if let Some((_cs, _ce, full_end)) = t569_string_literal_span(bytes, i) {
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
    panic!("T-566 Class-R: `{marker}` body not closed");
}

/// T-387 Class-R: each role must map to a distinct discord_id (and arma_id) in `dev.rs`.
///
/// Perturbation RED:
/// - delete a role-specific literal, OR
/// - collapse `discord_id_for_role` / `arma_id_for_role` so a role arm no longer binds its
///   dedicated constant (dead literals alone must not keep this green — measured hollow), OR
/// - move live match arms into `//` / `/* */` comments (T-566 W65 hollow — greps raw source), OR
/// - bind `discord_id = DEV_USER_ID` / `arma_id = DEV_ARMA_ID` at the call site while helpers
///   remain as dead code (T-566 ignore-helper hollow), OR
/// - park arms only inside `r#" "enlisted" => … "#` (or plain string) decoys while the live
///   match collapses to `_` (T-569 W66 hollow — comment strip keeps string contents).
///
/// Pre-T-387 a single `…001` / single arma literal was the measured fold.
///
/// # T-572 — the documented limit of this pin
///
/// **This test cannot decide whether the arms it finds are compiled**, and W67 walked around
/// T-569 with exactly that: `#[cfg(any())]` on the three role arms, live `_ => DEV_USER_ID`
/// underneath. The arms are still in the file, so the comment-strip + string-blank view still
/// sees them and this stays GREEN while every role folds back onto one row.
///
/// That was **not** fixed by teaching the view about `#[cfg(any())]`, and it should not be:
///
/// * the literal-`#[cfg(any())]` scrubber in `frontend/src/arsenal.rs` (T-503) is already known
///   to miss `#[cfg(all(unix, any()))]`, `#[cfg(feature = "nope")]` and plain extra whitespace —
///   a blocklist buys one wave;
/// * `if false`, `const NEVER: bool = false`, `std::hint::black_box(false)`, an early `return`,
///   a macro that expands to something else, and a `use other as …` shadow all reach the same
///   result with no attribute at all. Reachability is not a lexical property.
///
/// **Shapes this pin still admits (GREEN while the behaviour is gone):** any never-true `cfg` on
/// the arms or on either helper item; any constant-false guard around the `match`; a `return`
/// above it; macro-generated or `include!`d bodies. It is kept because it is fast, needs no
/// database, and names the exact literal that drifted — it is a first failure, not the contract.
///
/// **The contract is [`t572_dev_login_gives_every_role_its_own_identity_at_runtime`]**, which
/// logs in as all four roles and reads the identities back over HTTP.
#[test]
fn t387_dev_login_roles_use_distinct_discord_ids() {
    let handler = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers/dev.rs");
    let src = std::fs::read_to_string(&handler)
        .unwrap_or_else(|e| panic!("T-387 Class-R: read {}: {e}", handler.display()));
    // T-566: pins below run on comment-stripped code so `// "enlisted" => …` cannot green.
    let code = t566_strip_rust_comments_outside_literals(&src);

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
            "T-387: src/handlers/dev.rs missing discord_id `{id}` — each role needs its own row"
        );
    }
    for id in arma_ids {
        assert!(
            code.contains(id),
            "T-387: src/handlers/dev.rs missing arma_id `{id}` — per-role COALESCE must not race \
             idx_users_arma_id"
        );
    }
    assert!(
        code.contains("fn discord_id_for_role"),
        "T-387: expected discord_id_for_role helper — literals alone are not the contract"
    );
    assert!(
        code.contains("fn arma_id_for_role"),
        "T-387: expected arma_id_for_role helper — literals alone are not the contract"
    );

    // Live arms inside the helpers — comment-stripped (T-566) then non-pattern string
    // interiors blanked (T-569) so raw-string decoys cannot green.
    let discord_live = t569_live_match_view(t566_fn_body(&code, "discord_id_for_role"));
    let arma_live = t569_live_match_view(t566_fn_body(&code, "arma_id_for_role"));
    for arm in [
        "\"enlisted\" => DEV_USER_ID_ENLISTED",
        "\"leader\" => DEV_USER_ID_LEADER",
        "\"mission_maker\" => DEV_USER_ID_MISSION_MAKER",
    ] {
        assert!(
            discord_live.contains(arm),
            "T-387/T-566/T-569: expected live role arm `{arm}` inside discord_id_for_role \
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
            "T-387/T-566/T-569: expected live role arm `{arm}` inside arma_id_for_role \
             (comment-stripped + non-pattern strings blanked) — commented-out arms or \
             r#\" … \"# decoys reintroduce the single-id fold"
        );
    }

    // Call site must *use* the helpers (T-566 ignore-helper): dead helpers + DEV_USER_ID bind
    // must go red. Exact `let … = …(role);` — not the `fn …(role: &str)` signature.
    let login_body = t566_fn_body(&code, "dev_login");
    assert!(
        login_body.contains("let discord_id = discord_id_for_role(role);"),
        "T-387/T-566: dev_login must bind `let discord_id = discord_id_for_role(role);` — \
         `discord_id = DEV_USER_ID` with helpers left dead reintroduces the single-id fold"
    );
    assert!(
        login_body.contains("let arma_id = arma_id_for_role(role);"),
        "T-387/T-566: dev_login must bind `let arma_id = arma_id_for_role(role);` — \
         `arma_id = DEV_ARMA_ID` with helpers left dead races idx_users_arma_id again"
    );

    // Keep T-557/T-560 shape: NULL insert + live COALESCE (do not re-introduce INSERT stamp).
    // Soft pin here; T-560/T-562 in common/mod.rs own the sqlx::query-payload COALESCE gate.
    assert!(
        code.contains("SET arma_id = COALESCE(arma_id"),
        "T-387: must keep T-557/T-560 live COALESCE first-create path"
    );
    assert!(
        !src.contains("'', 'dev-arma-76561190000000001'"),
        "T-387: must not reintroduce fixed arma_id as INSERT VALUES literal"
    );
}

/// T-569: raw-string (and plain-string) arm decoys must not satisfy the match-arm pin;
/// live `"role" => CONST` arms must still be visible.
#[test]
fn t569_raw_string_arm_decoy_is_blanked_live_arms_kept() {
    // Build the W66 hollow shape via concat! (nested raw strings break a surrounding r#").
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

    // Sanity: pre-T-569 comment-strip-only view stays hollow-green on the decoy.
    for arm in arms {
        assert!(
            decoy.contains(arm),
            "fixture sanity: decoy must embed `{arm}` before blanking"
        );
    }

    let decoy_view = t569_live_match_view(decoy);
    let live_view = t569_live_match_view(live);
    for arm in arms {
        assert!(
            !decoy_view.contains(arm),
            "T-569: raw-string arm decoy must go RED after blanking; still found `{arm}` in:\n{decoy_view}"
        );
        assert!(
            live_view.contains(arm),
            "T-569: live match arm `{arm}` must stay visible (do not false-red valid binds)"
        );
    }

    // Comment-arm attack (T-566) still RED on the live-match view after comment strip.
    let commented = concat!(
        "fn discord_id_for_role(role: &str) -> &'static str {\n",
        "    match role {\n",
        "        // \"enlisted\" => DEV_USER_ID_ENLISTED,\n",
        "        /* \"leader\" => DEV_USER_ID_LEADER, */\n",
        "        _ => DEV_USER_ID,\n",
        "    }\n",
        "}\n",
    );
    let commented_view =
        t569_live_match_view(&t566_strip_rust_comments_outside_literals(commented));
    assert!(
        !commented_view.contains("\"enlisted\" => DEV_USER_ID_ENLISTED"),
        "T-566/T-569: // comment-arm must stay RED"
    );
    assert!(
        !commented_view.contains("\"leader\" => DEV_USER_ID_LEADER"),
        "T-566/T-569: /* */ comment-arm must stay RED"
    );
}

// ════════════════ §T-571 / T-572 — dev-login pinned by BEHAVIOUR, not by source ════════════════
//
// `common/mod.rs`'s COALESCE pin and `t387_dev_login_roles_use_distinct_discord_ids` below both
// read `src/handlers/dev.rs` as text. Six waves have now walked around one or the other, and the
// last two walk-arounds (W67) were not lexer bugs — `#[cfg(any())]` on the live match arms, and a
// nested `fn dev_login`, are questions about **reachability**, which no grep can answer.
//
// The three tests in this section answer it by running the handler against a real database. Dead
// code writes no rows, so every wrapper — the ones already invented and the ones nobody has —
// fails here by construction. The source pins stay as fast first failures; these are the contract.
//
// They share the per-role `users` rows, so they take one lock rather than reasoning about windows.
static DEV_ROLE_ROWS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `GET /auth/dev-login?role=…` → `(access token, /me discord_id, /me role)`.
///
/// Asserts the 302 itself: a 500 here is the `idx_users_arma_id` unique violation that a
/// collapsed `arma_id_for_role` produces on the second role's cold first create.
async fn dev_login_identity(app: &Router, role: &str) -> (String, String, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/dev-login?role={role}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND, "dev-login {role}");
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string();
    let me = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    let discord_id = v["user"]["discord_id"].as_str().unwrap().to_string();
    let reported_role = v["user"]["role"].as_str().unwrap().to_string();
    (tok, discord_id, reported_role)
}

/// `users.arma_id` for a discord id — `None` for "no row" and for "row with NULL".
async fn arma_id_of(pool: &PgPool, discord_id: &str) -> Option<String> {
    let row: Option<Option<String>> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(pool)
            .await
            .expect("read users.arma_id");
    row.flatten()
}

/// **T-571 — the COALESCE first-create contract, observed on the row.**
///
/// `common/mod.rs`'s Class-R greps `dev_login` for
/// `UPDATE users SET arma_id = COALESCE(arma_id, $2)`. W67 got that green with a nested
/// `fn dev_login` decoy and with a `$decoy$…$decoy$` payload; both are fixed there, and both
/// fixes are lexical, so the next verifier only has to reach for a `cfg` or an `if false`.
///
/// `COALESCE(arma_id, $2)` means exactly two observable things, and this asserts both:
///
/// * **NULL is filled.** That is the state the handler's own INSERT leaves (it binds NULL, and
///   its `ON CONFLICT` branch does not touch the column), so a role's first-ever dev-login must
///   come out with that role's arma id.
/// * **A set value is kept.** A user who has linked a real Arma id must still have it after the
///   next dev-login. This is the half `SET arma_id = $2` gets wrong.
///
/// Either observation alone is satisfiable by a handler with no UPDATE at all; together they are
/// not. Perturbation RED (measured, T-571):
/// - delete the UPDATE, or wrap it in `#[cfg(any())]` / `#[cfg(all(unix, any()))]` / `if false` →
///   step (A) fails: `arma_id` is still NULL.
/// - `SET arma_id = $2` → step (B) fails: the linked id was overwritten.
#[tokio::test]
async fn t571_dev_login_first_create_coalesces_arma_id() {
    let Some(app) = boot().await else {
        eprintln!("skip: test database URL unset");
        return;
    };
    let _guard = DEV_ROLE_ROWS.lock().await;
    let url = common::require_test_database_url().expect("boot succeeded ⇒ URL set");
    let pool = db::connect(&url).await.expect("connect");

    // `leader` is this binary's spare role — no sibling test asserts on its row.
    const ROLE: &str = "leader";
    const ID: &str = "000000000000000003";
    const ROLE_ARMA: &str = "dev-arma-76561190000000003";

    // (A) NULL arma_id — the post-INSERT state — must be stamped by the first-create UPDATE.
    sqlx::query("UPDATE users SET arma_id = NULL WHERE discord_id = $1")
        .bind(ID)
        .execute(&pool)
        .await
        .expect("clear arma_id");
    dev_login_identity(&app, ROLE).await;
    assert_eq!(
        arma_id_of(&pool, ID).await.as_deref(),
        Some(ROLE_ARMA),
        "T-571: dev-login left `{ID}` with no arma id. The first-create \
         `UPDATE users SET arma_id = COALESCE(arma_id, $2)` did not execute — deleted, `cfg`'d \
         out, wrapped in a constant-false block, or parked in a dead sibling helper. Source \
         still containing the statement is not the contract; this row is."
    );

    // (B) An already-linked arma id must survive. This is what COALESCE buys and what
    //     `SET arma_id = $2` destroys.
    let linked = common::unique_arma("t571-linked");
    sqlx::query("UPDATE users SET arma_id = $2 WHERE discord_id = $1")
        .bind(ID)
        .bind(&linked)
        .execute(&pool)
        .await
        .expect("link a real arma id");
    dev_login_identity(&app, ROLE).await;
    assert_eq!(
        arma_id_of(&pool, ID).await.as_deref(),
        Some(linked.as_str()),
        "T-571: dev-login overwrote a linked arma id. `COALESCE(arma_id, $2)` keeps what is \
         already there; a bare `SET arma_id = $2` is the regression this pins."
    );

    // Restore the row to the role's own id so a sibling test reads the shipped shape.
    sqlx::query("UPDATE users SET arma_id = NULL WHERE discord_id = $1")
        .bind(ID)
        .execute(&pool)
        .await
        .expect("clear arma_id");
    dev_login_identity(&app, ROLE).await;
    assert_eq!(
        arma_id_of(&pool, ID).await.as_deref(),
        Some(ROLE_ARMA),
        "T-571: re-stamp after unlink must reach the role's own arma id"
    );
}

/// **T-572 — every role's identity, observed over HTTP.**
///
/// [`t387_dev_login_roles_use_distinct_discord_ids`] greps `discord_id_for_role` /
/// `arma_id_for_role` for their live arms. W67 got that green with
/// `#[cfg(any())]` on the arms and a live `_ => DEV_USER_ID`: the arms are still *in the file*,
/// so every scrub-then-grep view still sees them, while the compiled `match` has one arm and
/// every role folds back onto the pre-T-387 shared row.
///
/// This asks the running handler instead. Four dev-logins, four `/me` identities:
///
/// * cfg'd-out **discord** arms → every role reports `…001` and the first `assert_eq!` fires.
/// * cfg'd-out **arma** arms → every role wants `dev-arma-…001`, so the second role's cold
///   first create trips the `idx_users_arma_id` unique index and dev-login answers 500 — the
///   302 assertion in [`dev_login_identity`] fires. (That index is the reason per-role arma ids
///   exist at all; T-557/T-387 are one contract.)
///
/// Perturbation RED (measured, T-572): `#[cfg(any())]` on the three role arms of either helper,
/// and `#[cfg(all(unix, any()))]` on them — the second walks around a literal-`#[cfg(any())]`
/// scrubber, and neither survives here.
#[tokio::test]
async fn t572_dev_login_gives_every_role_its_own_identity_at_runtime() {
    let Some(app) = boot().await else {
        eprintln!("skip: test database URL unset");
        return;
    };
    let _guard = DEV_ROLE_ROWS.lock().await;
    let url = common::require_test_database_url().expect("boot succeeded ⇒ URL set");
    let pool = db::connect(&url).await.expect("connect");

    const EXPECT: [(&str, &str, &str); 4] = [
        ("admin", "000000000000000001", "dev-arma-76561190000000001"),
        (
            "enlisted",
            "000000000000000002",
            "dev-arma-76561190000000002",
        ),
        ("leader", "000000000000000003", "dev-arma-76561190000000003"),
        (
            "mission_maker",
            "000000000000000004",
            "dev-arma-76561190000000004",
        ),
    ];

    let mut minted: Vec<String> = Vec::new();
    for (role, want_id, want_arma) in EXPECT {
        let (_tok, got_id, got_role) = dev_login_identity(&app, role).await;
        assert_eq!(
            got_id, want_id,
            "T-572: role `{role}` minted discord_id `{got_id}`, not its own `{want_id}`. Every \
             role folding onto one row is the pre-T-387 defect: `ON CONFLICT` rewrites that \
             row's role out from under the other roles and `issue_session` stacks refresh \
             families on it. A `cfg`-disabled match arm reads exactly like a live one in source."
        );
        assert_eq!(got_role, role, "T-572: `/me` role for `{role}`");
        assert_eq!(
            arma_id_of(&pool, want_id).await.as_deref(),
            Some(want_arma),
            "T-572: role `{role}` must own arma id `{want_arma}` — per-role ids are what keep \
             concurrent cold first creates off each other on `idx_users_arma_id`"
        );
        minted.push(got_id);
    }

    let mut distinct = minted.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        4,
        "T-572: four roles must mint four distinct identities; got {minted:?}"
    );
}

/// T-387 live: enlisted then admin must not rewrite each other's row / collide identity.
///
/// Perturbation RED: restore a single shared `DEV_USER_ID` for every role → after enlisted
/// login, admin login leaves one row whose role is admin, and enlisted's `/me` (JWT still
/// carries the shared id) reports admin — or the two `/me` discord_ids equal.
#[tokio::test]
async fn t387_dev_login_roles_do_not_rewrite_each_other() {
    let Some(app) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let _guard = DEV_ROLE_ROWS.lock().await;
    let url = common::require_test_database_url().expect("boot succeeded ⇒ URL set");
    let pool = db::connect(&url).await.expect("connect");

    let (_e_tok, e_id, e_role) = dev_login_identity(&app, "enlisted").await;
    assert_eq!(e_role, "enlisted");
    assert_eq!(e_id, "000000000000000002");

    let (_a_tok, a_id, a_role) = dev_login_identity(&app, "admin").await;
    assert_eq!(a_role, "admin");
    assert_eq!(a_id, "000000000000000001");
    assert_ne!(e_id, a_id, "roles must not share a discord_id");

    // Enlisted row must still be enlisted after admin login (the pre-T-387 rewrite).
    let enlisted_db_role: String =
        sqlx::query_scalar("SELECT role::text FROM users WHERE discord_id = $1")
            .bind(&e_id)
            .fetch_one(&pool)
            .await
            .expect("enlisted row");
    assert_eq!(
        enlisted_db_role, "enlisted",
        "admin login must not rewrite the enlisted row's role"
    );

    let (_m_tok, m_id, m_role) = dev_login_identity(&app, "mission_maker").await;
    assert_eq!(m_role, "mission_maker");
    assert_eq!(m_id, "000000000000000004");
    assert_ne!(m_id, e_id);
    assert_ne!(m_id, a_id);
}

#[tokio::test]
async fn request_id_echoed_and_honored() {
    let Some(app) = boot().await else { return };
    // No inbound id → the server generates one.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/announcements")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rid = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(!rid.is_empty(), "server assigns a request id");

    // Inbound id is honored.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/announcements")
                .header("x-request-id", "trace-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.headers()["x-request-id"], "trace-123");
}

#[tokio::test]
async fn cors_reflects_allowed_origin_only() {
    let Some(app) = boot().await else { return };
    // Allowed origin → reflected.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/announcements")
                .header(header::ORIGIN, ORIGIN)
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);

    // Disallowed origin → never reflected.
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/announcements")
                .header(header::ORIGIN, "http://evil.example")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let acao = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok());
    assert_ne!(acao, Some("http://evil.example"));
}

// ══════════════════════════ §T-235 — servers admin CRUD ══════════════════════════
//
// The ticket: no code path anywhere created a `servers` row. `INSERT INTO servers` existed only in
// three test files, there was no POST/PUT/DELETE route and no seed, so `GET /servers` returned an
// empty list on any production database forever and the Server Intel page had nothing to render.
//
// The constraint at the time: `handlers/servers.rs` was T-235's; `src/app.rs` — where routes are
// registered — was not. So the handoff was written as executable code (a `servers_crud_registration`
// router merged in when the probe found the routes unregistered) plus an assert-absent tripwire.
//
// T-586 landed the two `.route(...)` entries in `app::api_routes`, the tripwire fired exactly once
// as designed, and all three pieces — the merge shim, the `servers_crud_registered` probe and the
// tripwire test — are deleted per its instructions. `boot_servers` now hands every test below the
// production router, so these assertions additionally cover the request-id / logging / CORS /
// body-limit / rate-limit chain that the merge deliberately bypassed.

/// `(app, pool, state)` — mint whatever role tokens a test needs with [`token`].
///
/// Teardown is scoped to `name LIKE 'T235 %'` and is **never** a blanket `DELETE FROM servers`:
/// `telemetry.rs` and `admin_field.rs` insert their own `servers` rows into this same database and
/// cargo runs test binaries in parallel, so wiping the table here would fail their tests, not mine.
/// Every assertion below likewise filters `GET /servers` down to its own rows.
async fn boot_servers(tag: &str) -> Option<(Router, PgPool, AppState)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let like = format!("T235 {tag}%");
    sqlx::query(
        "DELETE FROM server_statuses WHERE server_id IN (SELECT id FROM servers WHERE name LIKE $1)",
    )
    .bind(&like)
    .execute(&pool)
    .await
    .expect("clean statuses");
    sqlx::query("DELETE FROM servers WHERE name LIKE $1")
        .bind(&like)
        .execute(&pool)
        .await
        .expect("clean servers");

    let state = AppState::new(pool.clone(), Config::for_tests(url, "servers-secret"));
    let app = app::router(state.clone());
    Some((app, pool, state))
}

/// Mint a bearer token for `role` **without** going through `/auth/dev-login`.
///
/// Pre-T-387, `dev_login` upserted ONE shared row (`…001`) and rewrote its `role`, so calling it
/// for `enlisted` here made `dev_login_unknown_role_defaults_to_admin` (asserts `/me` reports
/// admin) fail intermittently under parallel threads. T-387 gave each role its own discord_id,
/// so that rewrite landmine is gone — but these server-CRUD tests still mint a private JWT
/// (`…235`) that writes nothing, so they cannot perturb the real admin/enlisted/… rows. Coverage
/// is unchanged: `AdminUser` gates on the JWT's `role` claim (`middleware/auth.rs:80`), which is
/// exactly what this signs.
fn token(state: &AppState, role: &str) -> String {
    state
        .jwt
        .issue_access("000000000000000235", role, true)
        .expect("mint access token")
        .0
}

/// One request → `(status, parsed body)`. `bearer: None` sends no `Authorization` header.
async fn req(
    app: &Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let body = match body {
        Some(v) => {
            b = b.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = app.clone().oneshot(b.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The one row from `GET /servers` with this id, or `None`.
async fn list_row(app: &Router, bearer: &str, id: &str) -> Option<Value> {
    let (st, body) = req(app, Method::GET, "/api/v1/servers", Some(bearer), None).await;
    assert_eq!(st, StatusCode::OK, "GET /servers: {body}");
    body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .find(|r| r["id"] == id)
        .cloned()
}

/// Create → list → update → clear the modpack → deactivate → reactivate, every step asserted
/// through `GET /servers` because that is the endpoint the Server Intel page actually reads.
#[tokio::test]
async fn servers_crud_full_lifecycle() {
    let Some((app, pool, state)) = boot_servers("Life").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&state, "admin");

    // ── CREATE ───────────────────────────────────────────────────────────────────
    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "T235 Life Alpha", "ip": "10.20.30.40", "port": 2001 })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "POST /servers: {created}");
    let id = created["id"].as_str().expect("created id").to_string();
    assert_eq!(created["name"], "T235 Life Alpha");
    assert_eq!(created["ip"], "10.20.30.40");
    assert_eq!(created["port"], 2001);
    assert_eq!(created["is_active"], true, "is_active defaults true");
    assert!(
        created["status"].is_null(),
        "a fresh server has no telemetry"
    );
    assert!(
        created.get("required_modpack_id").is_none(),
        "absent, not null — matches `GET /servers` and dto.rs::ServerRowDto"
    );
    assert!(
        created.get("terrain").is_some_and(|t| t.is_null()),
        "T-385: fresh create has no current_match — terrain is explicit JSON null \
         (same encoding as status), never omitted via skip_serializing_if"
    );

    // ── LIST — the row the SPA renders ───────────────────────────────────────────
    let row = list_row(&app, &admin, &id).await.expect("row is listed");
    assert_eq!(
        row, created,
        "GET /servers serves exactly what POST returned"
    );

    // ── UPDATE — rename, re-address (IPv6), re-port ──────────────────────────────
    let (st, patched) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "name": "T235 Life Bravo", "ip": "0:0:0:0:0:0:0:1", "port": 2302 })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "PATCH: {patched}");
    assert_eq!(patched["name"], "T235 Life Bravo");
    assert_eq!(
        patched["ip"], "::1",
        "the address is canonicalised on write, so the stored value and the echo agree"
    );
    assert_eq!(patched["port"], 2302);
    assert_eq!(
        list_row(&app, &admin, &id).await.unwrap(),
        patched,
        "the list reflects the update"
    );

    // A patch naming one key leaves the rest alone.
    let (st, only_port) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "port": 2303 })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{only_port}");
    assert_eq!(only_port["port"], 2303);
    assert_eq!(only_port["name"], "T235 Life Bravo", "name untouched");
    assert_eq!(only_port["ip"], "::1", "ip untouched");

    // ── UPDATE — attach then clear the required modpack ───────────────────────────
    let modpack: Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, is_current, created_at) \
         VALUES ('T235 Life Pack', '1.0.0', 1024, false, now()) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed modpack");

    let (st, attached) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "required_modpack_id": modpack })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{attached}");
    assert_eq!(attached["required_modpack_id"], modpack.to_string());
    assert_eq!(
        attached["required_modpack"]["name"], "T235 Life Pack",
        "the modpack panel is composed into the row"
    );

    // An explicit `null` clears it; an *absent* key would have left it alone (the case the
    // `present_option` deserializer exists for — without it there would be no way to unset this).
    let (st, cleared) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "required_modpack_id": null })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{cleared}");
    assert!(
        cleared.get("required_modpack_id").is_none(),
        "cleared: {cleared}"
    );
    assert!(cleared.get("required_modpack").is_none(), "{cleared}");

    // ── DELETE = deactivate, and the row stays visible ───────────────────────────
    let (st, body) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT, "{body}");
    let row = list_row(&app, &admin, &id)
        .await
        .expect("a deactivated server is still listed — soft delete, not row removal");
    assert_eq!(row["is_active"], false);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM servers WHERE id = $1")
            .bind(Uuid::parse_str(&id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap(),
        1,
        "the row is still in the table"
    );

    // Idempotent: the end state is what was asked for, so a repeat is still 204.
    let (st, _) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT, "deactivate is idempotent");

    // ── REACTIVATE — the soft delete is reversible ────────────────────────────────
    let (st, revived) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "is_active": true })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{revived}");
    assert_eq!(revived["is_active"], true);

    // ── MISSING / MALFORMED ──────────────────────────────────────────────────────
    let ghost = Uuid::new_v4();
    for (method, want) in [
        (Method::PATCH, StatusCode::NOT_FOUND),
        (Method::DELETE, StatusCode::NOT_FOUND),
    ] {
        let (st, body) = req(
            &app,
            method.clone(),
            &format!("/api/v1/servers/{ghost}"),
            Some(&admin),
            Some(json!({ "port": 2400 })),
        )
        .await;
        assert_eq!(st, want, "{method} unknown id: {body}");
    }
    let (st, body) = req(
        &app,
        Method::PATCH,
        "/api/v1/servers/not-a-uuid",
        Some(&admin),
        Some(json!({ "port": 2400 })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "malformed id: {body}");

    // An empty patch is a client bug, not a no-op: `servers` has no `updated_at` to anchor the
    // SET list, so an unguarded empty patch would emit `UPDATE servers SET  WHERE …` — a syntax
    // error surfacing as 500.
    let (st, body) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "empty patch: {body}");

    sqlx::query("DELETE FROM modpacks WHERE id = $1")
        .bind(modpack)
        .execute(&pool)
        .await
        .ok();
}

/// T-535 / T-385 — **positive** live IT: `GET /servers.terrain` comes from the match JOIN.
///
/// Class-R — what absence makes this RED:
/// - Removing `LEFT JOIN matches m ON m.id = ss.current_match_id` (or `m.terrain AS terrain`)
///   from `SERVER_STATUS_SELECT_*` while hard-coding `terrain: None` / `NULL AS terrain`
///   → row still lists, but `terrain` stays JSON `null` despite a live `current_match_id`.
/// - Keeping only the create-path null assert (T-385) → that stays green either way.
///
/// Seed: match with `terrain = everon` + `server_statuses.current_match_id` → assert the list
/// JSON the Server Intel page reads equals `"everon"`.
#[tokio::test]
async fn servers_list_terrain_from_current_match_join() {
    let Some((app, pool, state)) = boot_servers("TerrainJoin").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&state, "admin");
    const SRC: &str = "t535-terrain-join";

    // Parallel-safe: wipe any leftover match from a prior crash (matches has no cascade from
    // servers, and boot_servers only cleans statuses/servers by T235 name prefix).
    sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
        .bind(SRC)
        .execute(&pool)
        .await
        .expect("clean prior T-535 match");

    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({
            "name": "T235 TerrainJoin Alpha",
            "ip": "10.53.5.1",
            "port": 5351
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "POST /servers: {created}");
    let id = created["id"].as_str().expect("created id").to_string();
    assert!(
        created.get("terrain").is_some_and(|t| t.is_null()),
        "precondition: fresh create still has terrain null before status+match seed"
    );

    let match_id: Uuid = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, terrain, started_at, outcome, created_at) \
         VALUES ($1, 'everon', now(), 'success', now()) RETURNING id",
    )
    .bind(SRC)
    .fetch_one(&pool)
    .await
    .expect("seed match with terrain=everon");

    let server_uuid = Uuid::parse_str(&id).expect("server uuid");
    sqlx::query(
        "INSERT INTO server_statuses \
         (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, \
          current_match_id, ingame_time, ingame_weather, updated_at) \
         VALUES ($1, true, 12, 64, 55.0, 900, $2, '06:42', 'overcast', now())",
    )
    .bind(server_uuid)
    .bind(match_id)
    .execute(&pool)
    .await
    .expect("wire server_statuses.current_match_id → match");

    // Assert on the list endpoint the Server Intel page reads — not create, not a unit pin.
    let row = list_row(&app, &admin, &id)
        .await
        .expect("seeded server must appear on GET /servers");
    assert_eq!(
        row["terrain"], "everon",
        "T-535: GET /servers.terrain must equal seeded matches.terrain via \
         LEFT JOIN on current_match_id — got {row}"
    );
    assert_eq!(
        row["status"]["current_match_id"],
        match_id.to_string(),
        "status must surface the wired current_match_id: {row}"
    );

    sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
        .bind(SRC)
        .execute(&pool)
        .await
        .ok();
}

/// The writes are admin-only; the reads stay member-tier. Asserted against the tier the handler's
/// own extractor enforces, so this holds however `app.rs` registers the routes.
#[tokio::test]
async fn servers_writes_are_admin_only() {
    let Some((app, _, state)) = boot_servers("Tier").await else {
        return;
    };
    let admin = token(&state, "admin");
    let (_, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "T235 Tier Alpha", "ip": "127.0.0.1", "port": 2101 })),
    )
    .await;
    let id = created["id"].as_str().expect("created id").to_string();

    // Reading is unchanged — every member tier still sees the Server Intel list.
    for role in ["enlisted", "leader", "mission_maker"] {
        let t = token(&state, role);
        let (st, _) = req(&app, Method::GET, "/api/v1/servers", Some(&t), None).await;
        assert_eq!(st, StatusCode::OK, "{role} reads stay member-tier");
    }

    // Writing is not — and `leader`/`mission_maker` matter as much as `enlisted`, because they
    // are the two tiers a "server config is close enough to mission config" registration would
    // plausibly have been given by mistake.
    for (method, uri, body) in [
        (
            Method::POST,
            "/api/v1/servers".to_string(),
            Some(json!({ "name": "T235 Tier Nope", "ip": "127.0.0.1", "port": 2102 })),
        ),
        (
            Method::PATCH,
            format!("/api/v1/servers/{id}"),
            Some(json!({ "name": "T235 Tier Nope" })),
        ),
        (Method::DELETE, format!("/api/v1/servers/{id}"), None),
    ] {
        for role in ["enlisted", "leader", "mission_maker"] {
            let t = token(&state, role);
            let (st, b) = req(&app, method.clone(), &uri, Some(&t), body.clone()).await;
            assert_eq!(
                st,
                StatusCode::FORBIDDEN,
                "{role} {method} {uri} must be refused: {b}"
            );
        }
        let (st, b) = req(&app, method.clone(), &uri, None, body).await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "anonymous {method} {uri} must be refused: {b}"
        );
    }

    // The refusals were refusals, not silent no-ops.
    let row = list_row(&app, &admin, &id).await.expect("row survives");
    assert_eq!(row["name"], "T235 Tier Alpha");
    assert_eq!(row["is_active"], true);
}

/// Every value the six-column table would have accepted and then broken something with, rejected
/// at the boundary with a 4xx. Each case is a **400 and not a 500** on purpose — see the doc
/// comments on `validated_ip` / `validated_port` / `require_modpack` for what the database did
/// before, which for `required_modpack_id` was worse than a 500: silent acceptance.
#[tokio::test]
async fn servers_write_validation_rejects_at_the_boundary() {
    let Some((app, pool, state)) = boot_servers("Valid").await else {
        return;
    };
    let admin = token(&state, "admin");

    let reject = |body: Value, why: &'static str| {
        let app = app.clone();
        let admin = admin.clone();
        async move {
            let (st, b) = req(
                &app,
                Method::POST,
                "/api/v1/servers",
                Some(&admin),
                Some(body),
            )
            .await;
            assert_eq!(st, StatusCode::BAD_REQUEST, "{why}: {b}");
            assert!(b["error"].is_string(), "{why}: envelope carries a message");
        }
    };

    // `ip` is Postgres `inet`. A hostname raises SQLSTATE 22P02, which `From<sqlx::Error>` turns
    // into a logged 500 — verified against the live DB.
    reject(
        json!({ "name": "T235 Valid A", "ip": "tbd.example.com", "port": 2201 }),
        "hostname",
    )
    .await;
    // `host('10.0.0.5/24'::inet)` = `10.0.0.5` (measured), so a mask would be accepted and then
    // silently altered — the stored address would differ from the one sent.
    reject(
        json!({ "name": "T235 Valid A", "ip": "10.0.0.5/24", "port": 2201 }),
        "cidr mask",
    )
    .await;
    reject(
        json!({ "name": "T235 Valid A", "ip": "", "port": 2201 }),
        "empty ip",
    )
    .await;
    reject(
        json!({ "name": "T235 Valid A", "ip": "999.1.1.1", "port": 2201 }),
        "not an address",
    )
    .await;

    // `port` is `bigint` with no CHECK: 0, -1 and 999999999 all store fine and then render as an
    // address nothing can connect to.
    for bad in [0, -1, 65536, 999_999_999_i64] {
        reject(
            json!({ "name": "T235 Valid A", "ip": "127.0.0.1", "port": bad }),
            "port out of range",
        )
        .await;
    }

    // `name` is `text NOT NULL` with no CHECK, and the card has no other identifier on it.
    reject(
        json!({ "name": "", "ip": "127.0.0.1", "port": 2201 }),
        "empty name",
    )
    .await;
    reject(
        json!({ "name": "   ", "ip": "127.0.0.1", "port": 2201 }),
        "whitespace-only name",
    )
    .await;

    // Required on create, and the message names the field rather than falling through to axum's.
    reject(json!({ "ip": "127.0.0.1", "port": 2201 }), "no name").await;
    reject(json!({ "name": "T235 Valid A", "port": 2201 }), "no ip").await;
    reject(
        json!({ "name": "T235 Valid A", "ip": "127.0.0.1" }),
        "no port",
    )
    .await;

    // `required_modpack_id` has no foreign key, so an unknown id used to store silently and the
    // card just lost its modpack panel with nothing complaining anywhere.
    reject(
        json!({
            "name": "T235 Valid A", "ip": "127.0.0.1", "port": 2201,
            "required_modpack_id": Uuid::new_v4()
        }),
        "unknown modpack",
    )
    .await;

    // A malformed body is a 400 whose `details.reason` names the offending field. Before
    // `body_error` this answered "name, ip and port are required" — three fields that were all
    // present and correct.
    let (st, b) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(
            json!({ "name": "T235 Valid A", "ip": "127.0.0.1", "port": 2201,
                     "required_modpack_id": "not-a-uuid" }),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{b}");
    assert!(
        b["details"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("required_modpack_id"),
        "the 400 must name the field that failed to parse: {b}"
    );

    // Nothing above was stored.
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM servers WHERE name LIKE 'T235 Valid%'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0,
        "a rejected create must not have written a row"
    );

    // What IS accepted: the name is trimmed once and stored trimmed (read and write agree), and
    // the address is canonicalised, so `RETURNING host(ip)` echoes what is really in the column.
    let (st, ok) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "  T235 Valid Trimmed  ", "ip": " ::ffff:1.2.3.4 ", "port": 65535 })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{ok}");
    assert_eq!(ok["name"], "T235 Valid Trimmed");
    assert_eq!(ok["port"], 65535);
    let id = ok["id"].as_str().unwrap().to_string();
    assert_eq!(
        list_row(&app, &admin, &id).await.unwrap()["ip"],
        ok["ip"],
        "the address the create echoed is the address the list serves"
    );

    // PATCH runs the same validators, and a rejected patch changes nothing.
    for bad in [
        json!({ "ip": "tbd.example.com" }),
        json!({ "port": 0 }),
        json!({ "name": "  " }),
        json!({ "required_modpack_id": Uuid::new_v4() }),
    ] {
        let (st, b) = req(
            &app,
            Method::PATCH,
            &format!("/api/v1/servers/{id}"),
            Some(&admin),
            Some(bad.clone()),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "PATCH {bad}: {b}");
    }
    let after = list_row(&app, &admin, &id).await.unwrap();
    assert_eq!(after["name"], "T235 Valid Trimmed", "unchanged: {after}");
    assert_eq!(after["port"], 65535, "unchanged: {after}");
}

// T-586: the tripwire `servers_crud_registration_pending_in_app_rs` lived here and fired, exactly
// once, as designed. `app.rs` now registers the write routes, so the merge shim, the probe and the
// tripwire are all deleted per its own failure message, and the lifecycle tests above drive the
// PRODUCTION router — including the middleware chain the shim used to bypass.
