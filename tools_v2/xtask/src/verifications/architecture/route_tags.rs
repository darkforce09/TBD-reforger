//! GO-7 — every `@route` tag resolves to a registered Axum route, and every route back to a tag
//! (T-853 port of `scripts/verify-route-tags.sh`, the check restored at T-586/T-590).
//!
//! ── WHY A SCRIPT HAD TO EXIST AT ALL ─────────────────────────────────────────────────────────
//!
//! CODING_STANDARDS.md GO-7: *"Every exported handler func SHALL carry `@route` in its Godoc, and
//! the tag MUST match the wired route in `handlers.go` `Register()` (method + path)."* §2 of that
//! document classes GO-7 **CI-SCRIPT** rather than lint-enforced, and that is the whole reason:
//! clippy does not read doc comments and `cargo fmt` only reflows them, so nothing in the compiler
//! can see a comment claiming a route. The Go gate was `verify-contract-citations.mjs` — presence
//! AND route-match across all 82 Go handlers. T-145's Go→Rust rewrite deleted `Register()` and
//! every Go handler, GO-7 died with them, and nothing replaced it: `Makefile:304` still claimed the
//! GO-2..9 analogs were "enforced by clippy + the centralized ApiError type + `cargo fmt`", none of
//! which can see a doc comment.
//!
//! MEASURED CONSEQUENCE (T-586, found by T-576): `handlers/servers.rs` carried `@route` tags on
//! THREE handlers — `create_server` (POST), `update_server` (PATCH), `deactivate_server` (DELETE) —
//! that no route table registered. The whole admin server-CRUD triple was documented, tested and
//! unreachable, and nothing went red. In the other direction `submit_mission` was a live registered
//! route carrying no tag at all. A documentation tag nobody checks is a claim, not a contract.
//!
//! ── THE TWO DIRECTIONS, BOTH HARD FAILURES ───────────────────────────────────────────────────
//!
//!   A. TAG → ROUTER  every `@route METHOD PATH` must be registered, on that method, for that
//!      handler — the T-586 triple: a claim to a door that is not in the wall.
//!   B. ROUTER → TAG  every registered route must carry a matching `@route` on the handler it
//!      names. GO-7's "presence" half, and not optional: DOCUMENTATION_STANDARDS.md §3.1 makes
//!      `@route` REQUIRED on the serving handler, one leg of the three-way triangulation a mod
//!      author greps.
//!
//! Both keys are (METHOD, PATH, HANDLER FN), not just the path — which is what makes a tag moved
//! onto the wrong handler, or a handler rewired elsewhere, fail as loudly as one never wired.
//!
//! ── WHERE THE ROUTES LIVE ────────────────────────────────────────────────────────────────────
//!
//! The registrations are not in one function. Each domain owns a route table at
//! `src/<domain>/routes.rs`, holding exactly one column-0 `pub fn routes`, and `http_router.rs`'s
//! [`MERGE_FN`] merges all of them under [`API_PREFIX`]. So the router side of this check is the
//! UNION of every discovered table, and `http_router.rs` is read only for its shape.
//!
//! ── VACUITY GUARDS (T-556: a gate reporting nothing == a gate checking nothing) ───────────────
//!
//! A verifier that passes because it parsed zero inputs is the T-586 defect in a new hat, so the
//! parse is checked against itself before any verdict is issued: every raw `@route` line must
//! become exactly one parsed tuple; every `.route(` line must yield at least one registration;
//! every discovered route table must be merged and every merged table must exist on disk;
//! `http_router.rs` must still have the shape the extractor parses; and four sentinel routes
//! present on both sides must survive the pipeline. Each is a FAIL, never a SKIP.
//!
//! The mount cross-check is the guard that the split into tables made necessary: a table nobody
//! merges serves nothing while still supplying routes to side B, and a merge with no table behind
//! it is a build error here but a silently shrinking router side if this gate ever ran on a
//! partial checkout. Both directions are named, one FAIL line each.
//!
//! ── WHAT THE PORT FIXES ──────────────────────────────────────────────────────────────────────
//!
//! 1. **Exit 127 is unreachable for the matcher.** The script's header warns at length about search
//!    tools that answer differently depending on WHO invoked them: `rg` is installed nowhere here
//!    and exists in an agent shell only as an injected function (T-556), and one layer down `grep`
//!    is *ugrep 7.5.0* as an agent-shell function but GNU grep 3.8 under `bash script.sh` (measured
//!    2026-07-31, T-586). ugrep rejects an unescaped `{` in an ERE ("invalid repeat", exit 2) where
//!    GNU grep takes it literally — and **every route path here contains `{id}`**. bash survived by
//!    routing every route-shaped comparison through `-F`. Here the engine is the `regex` crate
//!    compiled in: no `PATH`, no shell function, no skew. [`Pattern::literal`] is kept wherever
//!    bash wrote `-F`, so the mapping stays reviewable 1:1.
//! 2. **`2>/dev/null || true` on the tag sweep is closed.** It turned "the handlers tree moved"
//!    into an empty file list. The script did not go green on that — its own vacuity guard caught
//!    the zero — but it then reported *"parsed NOTHING"* when the truth was *"I could not read the
//!    tree"*, which sends the reader to the wrong file. [`scan::walk_files`] makes it a `NotRun`.
//! 3. **Deterministic ordering.** MEASURED 2026-08-12: the script's `sort`s run under the ambient
//!    locale and no caller pins one (nothing matches `LC_ALL|LC_COLLATE` in `Makefile`,
//!    `scripts/platform/wave.sh` or `.github/workflows/`). Under `LANG=en_AU.UTF-8` glibc ignores
//!    punctuation at the primary level, so `DELETE …/{id}/bookmark` lists BEFORE `DELETE …/{id}`;
//!    under `LANG=C` it lists after — so the report order depended on the operator's environment,
//!    rule 1 of `.cursor/rules/acceptance-gates-reproducible.mdc`, in the one script whose header is
//!    a sermon against exactly that. [`collate_cmp()`] reproduces the measured `en_AU.UTF-8` order
//!    with no locale input at all: the same bytes on every machine, and the same bytes the
//!    committed baseline was captured with.
//!
//! ── DELIBERATE DEVIATIONS (everything else is byte-for-byte) ─────────────────────────────────
//!
//! * **[`SELF_REL`] in the two shape-pin messages** tells the reader where the extractor they
//!   must re-point lives, which is this file. Reachable only once [`MERGE_FN`] has been renamed —
//!   never on a clean tree.
//! * **Exit 2, not 1, when the check DID NOT RUN** (missing or unreadable `http_router.rs` or
//!   `src/`), as in the other verifications that separate the two. Callers that only test
//!   `rc == 0` still read FAIL, and the headline stays on line 1 so a grep for
//!   `FAIL: http_router.rs no longer …` still hits. A real A/B violation exits **1**.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use verification_core::{Kind, NotRun, Pattern, Verdict, gate, scan};

/// The router assembly. Relative, because the script `cd`s to `$ROOT` and printed relative paths.
/// Read for its SHAPE only — the registrations live in the domain tables it merges.
const ROUTER_RS_REL: &str = "apps/website/api_v2/src/core/http_router.rs";
/// The tree swept for `@route` tags — the whole `src/`, not just `handlers/`. Also the tree the
/// domain route tables are discovered in.
const SRC_DIR_REL: &str = "apps/website/api_v2/src";
/// The nest prefix every `@route` tag is written against. Asserted, never assumed: if
/// `http_router.rs`
/// stops nesting the merged tables here, every extracted path is silently wrong.
const API_PREFIX: &str = "/api/v1";
/// The function in `http_router.rs` that merges the domain route tables. Its body is read for the
/// `.merge(crate::<domain>::routes(` lines the mount cross-check compares against the tree.
const MERGE_FN: &str = "fn api_v1_routes";
/// [`MERGE_FN`] without the `fn` keyword, for the messages that NAME the function rather than pin
/// its declaration.
const MERGE_FN_NAME: &str = "api_v1_routes";
/// A domain route table is exactly `src/<domain>/routes.rs` — one directory level below `src`.
/// Anything deeper is a handler, a model or a test, and is swept for tags but never for routes.
const ROUTES_FILE: &str = "routes.rs";
/// The column-0 function each route table declares exactly once. The `(` is part of the needle so
/// a neighbouring `pub fn routes_for_tests(` cannot be mistaken for it.
const ROUTES_FN: &str = "pub fn routes(";
/// How the report names the router side, which is now a set of files rather than one function.
const ROUTE_TABLES: &str = "the api_v2 domain route tables";
/// bash interpolated `$0`. See the module docs on the one deliberate text deviation.
const SELF_REL: &str = "tools_v2/xtask/src/verifications/architecture/route_tags.rs";
/// Routes registered AND tagged today, spanning three separate domain tables and covering a path
/// parameter, a bare path and a chained method. If the extractor breaks in a way the counting
/// guards miss, these vanish and the run fails rather than quietly comparing two short lists that
/// happen to agree. Spanning several files is the point: a discovery bug that finds only one table
/// still satisfies a single-file sentinel set.
const SENTINELS: &[&str] = &[
    "GET /api/v1/servers list_servers",
    "GET /api/v1/servers/{id}/status get_server_status",
    "POST /api/v1/events create_event",
    "GET /api/v1/me get_me",
];

// Fixed output blocks, as consts because rustfmt cannot break a string literal — and every byte
// here is contract: `wave.sh` scrapes these logs and T-853 accepts ports by diffing stdout.
const SHAPE_FAIL: &str = "ROUTE-TAG CHECK: FAIL (router shape changed — the extractor was not run)";
const PARSE_FAIL: &str =
    "ROUTE-TAG CHECK: FAIL (the parse could not be trusted — no tag/route verdict was issued)";
const PROBE_FAIL: &[&str] = &[
    "FAIL: grep self-probe returned 1 over a subject it must match.",
    "      The search engine is broken or missing. A check that cannot run is not a pass.",
];
const NOTHING_TAIL: &[&str] = &[
    "      A route-tag check with no inputs is not a pass. Either the crate moved or this",
    "      script's extractor is broken; both are red.",
];
const ORPHAN_TAIL: &str =
    "      A tag with no handler beneath it, or a malformed tag, is an unreadable claim.";
const UNPARSED_TAIL: &str =
    "      A registration shape this extractor cannot read must not be silently skipped.";
const MOUNT_TAIL: &str =
    "      A route table and the merge that serves it are two halves of one registration.";
const VERDICT_TAIL: &[&str] = &[
    "  A @route tag is a contract with the router, not a comment. Wire the route, move the",
    "  tag onto the handler that really serves it, or delete the claim.",
];

// ── Extraction ───────────────────────────────────────────────────────────────────────────────

/// Both patterns are anchored at column 0, exactly as the awk wrote them: an indented `/// @route`
/// inside an `impl` block is invisible to this gate. Measured 2026-08-12 there are none
/// (`grep -rhE '^[[:space:]]+///[[:space:]]*@route'` → 0), so the anchor costs nothing today and is
/// preserved rather than widened, because widening it is a behaviour change, not a port.
const TAG_RE: &str = r"^///[[:space:]]*@route[[:space:]]";
const FN_RE: &str = r"^pub[[:space:]]+(async[[:space:]]+)?fn[[:space:]]";

// ── Small helpers, one shell construct each ──────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/route_tags/tests.rs"]
mod tests;

mod verify_route_tags;
pub use verify_route_tags::verify_route_tags;

mod route_and_tag_extraction;
use route_and_tag_extraction::{
    discover_route_files, extract_all_tags, extract_router, flatten, merged_domains,
    routes_fn_lines,
};

mod collate_cmp;
use collate_cmp::collate_cmp;

#[cfg(test)]
use route_and_tag_extraction::extract_tags;
#[cfg(test)]
use verify_route_tags::run;
