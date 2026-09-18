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
//! that `http_router.rs` never registered. The whole admin server-CRUD triple was documented, tested and
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
//! ── VACUITY GUARDS (T-556: a gate reporting nothing == a gate checking nothing) ───────────────
//!
//! A verifier that passes because it parsed zero inputs is the T-586 defect in a new hat, so the
//! parse is checked against itself before any verdict is issued: every raw `@route` line must
//! become exactly one parsed tuple; every `.route(` line must yield at least one registration;
//! `http_router.rs` must still have the shape the extractor parses; and two sentinel routes present on both
//! sides must survive the pipeline. Each is a FAIL, never a SKIP.
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
//! * **`$0` in the two shape-pin messages** becomes [`SELF_REL`]. That sentence tells the reader
//!   where the extractor they must re-point lives, and after T-853 that is this file; naming a
//!   script the migration removes would be actively misleading. Reachable only once `fn api_routes`
//!   has been renamed — never on a clean tree.
//! * **Exit 2, not 1, when the check DID NOT RUN** (missing/unreadable `http_router.rs` or `src/`), as in
//!   `sql_gates.rs` and `gate_t439.rs`. `Makefile:328` and `wave.sh:2562`/`:2833` test `rc -eq 0`,
//!   so any nonzero is still FAIL there, and the bash headline stays verbatim on line 1 so a grep
//!   for `FAIL: http_router.rs no longer …` still hits. A real A/B violation still exits **1**.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use verification_core::{Kind, NotRun, Pattern, Verdict, gate, scan};

/// The router. Relative, because the script `cd`s to `$ROOT` and printed relative paths.
const APP_RS_REL: &str = "apps/website/api_v2/src/core/http_router.rs";
/// The tree swept for `@route` tags — the whole `src/`, not just `handlers/`.
const SRC_DIR_REL: &str = "apps/website/api_v2/src";
/// The nest prefix every `@route` tag is written against. Asserted, never assumed: if
/// `http_router.rs`
/// stops nesting `api_routes` here, every extracted path is silently wrong.
const API_PREFIX: &str = "/api/v1";
/// bash interpolated `$0`. See the module docs on the one deliberate text deviation.
const SELF_REL: &str = "tools_v2/xtask/src/verifications/architecture/route_tags.rs";
/// Two routes registered AND tagged today, one with a path parameter and one without. If the
/// extractor breaks in a way the counting guards miss, these vanish and the run fails rather than
/// quietly comparing two short lists that happen to agree.
const SENTINELS: &[&str] = &[
    "GET /api/v1/servers list_servers",
    "GET /api/v1/servers/{id}/status get_server_status",
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

mod collate_cmp;
use collate_cmp::collate_cmp;

#[cfg(test)]
use verify_route_tags::{api_routes_lines, extract_router, extract_tags, flatten, run};
