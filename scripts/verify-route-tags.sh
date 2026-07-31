#!/usr/bin/env bash
# verify-route-tags.sh — GO-7, restored for the Axum crate (T-586).
#
# ── What this checks, and why it did not exist ───────────────────────────────────────────────────
#
# CODING_STANDARDS.md GO-7: "Every exported handler func SHALL carry `@route` in its Godoc, and the
# tag MUST match the wired route in `handlers.go` `Register()` (method + path)." It was a CI-SCRIPT
# gate — `verify-contract-citations.mjs`, presence AND route-match across all 82 Go handlers.
#
# The T-145 Go→Rust rewrite deleted `Register()` and every Go handler, and GO-7 died with them.
# Nothing replaced it. `Makefile:304` still claims the GO-2..9 analogs are "enforced by clippy + the
# centralized ApiError type + `cargo fmt`" — none of which can see a doc comment claiming a route.
#
# The measured consequence (T-586, found by T-576): `handlers/servers.rs` carried `@route` tags on
# THREE handlers — `create_server` (POST), `update_server` (PATCH), `deactivate_server` (DELETE) —
# that `app.rs` never registered. The whole admin server-CRUD triple was documented, tested, and
# unreachable, and nothing anywhere went red for it. In the other direction `submit_mission` was a
# live registered route carrying no tag at all.
#
# A documentation tag nobody checks is a claim, not a contract. This script is the check.
#
# ── The two directions, both hard failures ──────────────────────────────────────────────────────
#
#   A. TAG → ROUTER   every `@route METHOD PATH` must be registered, on that method, for that
#                     handler function. Catches the T-586 triple: a claim to a door that is not
#                     in the wall.
#   B. ROUTER → TAG   every registered route must carry a matching `@route` on the handler it
#                     names. Catches the inverse: an undocumented door. This is GO-7's "presence"
#                     half and it is not optional — DOCUMENTATION_STANDARDS.md §3.1 makes `@route`
#                     REQUIRED on the handler that serves the route, because it is one leg of the
#                     three-way triangulation a mod author greps.
#
# Both keys are (METHOD, PATH, HANDLER FN) — not just the path. Keying on the function name is what
# makes a tag moved onto the wrong handler, or a handler rewired to a different path, fail as loudly
# as one that is not wired at all.
#
# ── Why `grep -E` and not `rg`, and why braces are ESCAPED ───────────────────────────────────────
#
# `rg` is installed NOWHERE on this machine (T-556). `command -v rg` succeeds in an agent shell only
# because Claude Code injects a shell FUNCTION; functions do not survive into a subshell, so an
# rg-based gate passes only when an AI runs it.
#
# MEASURED 2026-07-31, T-586 — the same hazard exists one layer down, for `grep`, with the polarity
# REVERSED, and it is not documented anywhere else:
#
#   in an agent shell            `type grep` -> function -> ugrep 7.5.0
#   under `bash thisscript.sh`   `command -v grep` -> /usr/bin/grep -> GNU grep 3.8
#
# ugrep REJECTS an unescaped `{` in an ERE — `^GET /a/{id}$` is "invalid repeat", exit **2** — where
# GNU grep accepts it as a literal. Every route path in this repo contains `{id}`. So an ERE pattern
# with a bare brace is green under the gate and a hard error when a human pastes it into a shell,
# which is exactly the "different verdict depending on WHO invoked it" failure rule 1 of
# `.cursor/rules/acceptance-gates-reproducible.mdc` forbids. Both engines accept `\{`.
#
# Therefore: every route-shaped comparison in here goes through `-F` (literal), and the one ERE that
# must contain a brace escapes it. This script produces the same verdict under either engine.
#
# ── Fail-closed, via scripts/mod/lib/gate-grep.sh ────────────────────────────────────────────────
#
# The library's whole subject is that a boolean cannot carry four outcomes:
#   0 match · 1 no match · 2 target missing / pattern error · 127 tool absent
# The last two are checks that DID NOT RUN, and both fail closed here, naming which happened. This
# repo has fixed fail-open shell guards twice; this is not the third.
#
# ── And the vacuity guards, because this script is its own signature defect risk ─────────────────
#
# A route-tag verifier that passes because it parsed nothing is the T-586 defect wearing a new hat.
# So the parse is checked against itself before any verdict is issued:
#
#   * every raw `@route` line in the tree must become exactly one parsed (METHOD, PATH, FN) tuple —
#     a tag with no handler under it is named, not skipped;
#   * every `.route(` in `api_routes` must yield at least one (METHOD, PATH, FN) registration —
#     a registration shape the extractor cannot read is named, not skipped;
#   * `app.rs` must still have the shape this extractor parses (`fn api_routes`, nested at
#     `/api/v1`) — a restructured router makes the extractor wrong, not lenient;
#   * two sentinel routes that exist on both sides must survive the whole pipeline.
#
# Any of those failing is a FAIL, not a SKIP. Prove it: append a `@route` tag pointing at nothing,
# or delete a `.route(...)` line, and this must go red.
#
# Usage:  bash scripts/verify-route-tags.sh
# Exit:   0 = every @route tag and every registered route agree.  1 = they do not, or the check
#         could not run.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APP_RS="apps/website/api/src/app.rs"
SRC_DIR="apps/website/api/src"
LIB="scripts/mod/lib/gate-grep.sh"

# The nest prefix every `@route` tag in the crate is written against. Asserted below, not assumed:
# if `app.rs` stops nesting `api_routes` here, the extracted paths are silently wrong.
API_PREFIX="/api/v1"

# Two routes that are registered AND tagged today, one with a path parameter and one without. If
# the extractor breaks in a way the counting guards do not catch, these disappear and the run fails
# rather than quietly comparing two short lists that happen to agree.
SENTINELS=(
	"GET /api/v1/servers list_servers"
	"GET /api/v1/servers/{id}/status get_server_status"
)

fail=0
note() { echo "FAIL: $*"; fail=1; }

# ── The library, and proof its engine is actually present ────────────────────────────────────────

if [ ! -f "$LIB" ]; then
	echo "FAIL: gate helper library missing: $LIB"
	echo "      The four-outcome grep helpers could not be loaded, so nothing below can be trusted."
	exit 1
fi
# shellcheck source=scripts/mod/lib/gate-grep.sh
. "$LIB"

# Probe the search tool over a subject whose answer is known, BEFORE it is used for a verdict.
# 0 is the only acceptable status: 1 would mean grep ran and disagreed with arithmetic, 127 that it
# is absent, 2 that it errored. Only 0 proves the engine works.
probe="$(gate_probe_str -F "tbd" "tbd-reforger")"
case "$probe" in
0) ;;
127)
	echo "FAIL: the search tool is ABSENT (grep exited 127)."
	echo "      Refusing to report OK on a route-tag check that did not execute."
	exit 1
	;;
*)
	echo "FAIL: grep self-probe returned $probe over a subject it must match."
	echo "      The search engine is broken or missing. A check that cannot run is not a pass."
	exit 1
	;;
esac

# ── Shape assertions on app.rs ───────────────────────────────────────────────────────────────────
#
# The extractor below reads ONE function and prefixes ONE nest path. Both are load-bearing, so both
# are pinned. `gate_require` covers the missing-file and tool-error cases on the way through.

gate_require "app.rs no longer defines \`fn api_routes\` — the route extractor in $0 reads that function by name, so it is now parsing nothing. Re-point it before trusting any verdict." \
	-F 'fn api_routes' "$APP_RS" || fail=1

gate_require "app.rs no longer nests api_routes at \`$API_PREFIX\` — every @route tag in the crate is written with that prefix, so the extracted paths would all be wrong." \
	-F ".nest(\"$API_PREFIX\", api_routes(" "$APP_RS" || fail=1

if [ "$fail" -ne 0 ]; then
	echo
	echo "ROUTE-TAG CHECK: FAIL (router shape changed — the extractor was not run)"
	exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── Extract the router: (METHOD, PATH, HANDLER) for every registration in api_routes ─────────────
#
# `api_routes` is taken from its `fn` line to the next column-0 `}` (the crate is rustfmt-clean, so
# that is the function's own closing brace). `//` line comments are stripped so a commented-out
# `.route(...)` cannot be read as live, and the body is flattened to one line because rustfmt splits
# a single registration across up to five.
#
# The flattened body is then split on the literal `.route(`. Each piece holds exactly one
# registration: its path is the first quoted string, and its method/handler pairs are every
# `method(path::to::fn` in it — which is what makes the chained `get(a).post(b)` and the
# `axum::routing::patch(a).delete(b)` forms both fall out correctly. Splitting on `.route(` cannot
# catch `.route_layer(` (the next character is `_`, not `(`).
#
# A piece that yields no method, or no path, is emitted as an UNPARSED marker rather than dropped.
# That is the difference between this and a check that silently shrinks.
sed -n '/^fn api_routes/,/^}/p' "$APP_RS" \
	| sed 's://.*$::' \
	| tr '\n' ' ' \
	| awk -v prefix="$API_PREFIX" '
	BEGIN { RS = "\\.route\\(" }
	NR == 1 { next }   # everything before the first .route( is the fn signature
	{
		if (match($0, /"[^"]*"/)) { p = substr($0, RSTART + 1, RLENGTH - 2) }
		else { print "UNPARSED no-path-literal-in-registration-" NR; next }
		s = $0; n = 0
		while (match(s, /(get|post|put|patch|delete|head|options|trace)\([ ]*[A-Za-z_:0-9]+/)) {
			m = substr(s, RSTART, RLENGTH); s = substr(s, RSTART + RLENGTH)
			split(m, a, "("); meth = a[1]
			gsub(/^ +| +$/, "", a[2]); k = split(a[2], b, "::"); fn = b[k]
			print toupper(meth) " " prefix p " " fn
			n++
		}
		if (n == 0) { print "UNPARSED no-method-handler-for-path-" p }
	}' | sort > "$TMP/router.txt"

# ── Extract the tags: (METHOD, PATH, HANDLER) for every @route in the crate ──────────────────────
#
# A tag binds to the next `pub fn` / `pub async fn` below it. A tag with no handler under it is an
# ORPHAN — emitted, never dropped, because that is a malformed claim and the vacuity guard's job is
# to notice claims this parser could not read.
#
# `:id` is normalised to `{id}` (name preserved, so `:id` documented against `{mission_id}` wired
# still fails). The ERE that does it escapes nothing, but the SUBSTITUTION target contains braces —
# awk's ERE, not grep's, so the ugrep/GNU split above does not apply here.
: > "$TMP/tags.txt"
tag_files="$(grep -rlE '^///[[:space:]]*@route[[:space:]]' "$SRC_DIR" --include='*.rs' 2>/dev/null || true)"
for f in $tag_files; do
	awk -v F="$f" '
		function emit_orphan() { print "ORPHAN " F ":" pline " " pm " " pp }
		/^\/\/\/[[:space:]]*@route[[:space:]]/ {
			if (pend) emit_orphan()
			line = $0
			sub(/^\/\/\/[[:space:]]*@route[[:space:]]+/, "", line)
			n = split(line, a, /[[:space:]]+/)
			pm = toupper(a[1]); pp = a[2]
			if (n < 2 || pm == "" || pp == "") { print "ORPHAN " F ":" NR " malformed-tag"; pend = 0; next }
			gsub(/:[A-Za-z_][A-Za-z_0-9]*/, "{&}", pp)   # :id -> {:id}
			gsub(/\{:/, "{", pp)                          # {:id} -> {id}
			pend = 1; pline = NR; next
		}
		/^pub[[:space:]]+(async[[:space:]]+)?fn[[:space:]]/ {
			if (pend) {
				l = $0
				sub(/^pub[[:space:]]+(async[[:space:]]+)?fn[[:space:]]+/, "", l)
				sub(/[(<].*$/, "", l)
				print pm " " pp " " l " " F ":" pline
				pend = 0
			}
		}
		END { if (pend) emit_orphan() }
	' "$f" >> "$TMP/tags.txt"
done
sort -o "$TMP/tags.txt" "$TMP/tags.txt"

# ── Vacuity guards — run BEFORE any verdict, because a verdict over an empty parse is the defect ─

router_bad="$(grep -c '^UNPARSED ' "$TMP/router.txt" || true)"
tag_bad="$(grep -c '^ORPHAN ' "$TMP/tags.txt" || true)"
n_routes="$(($(wc -l < "$TMP/router.txt") - router_bad))"
n_tags="$(($(wc -l < "$TMP/tags.txt") - tag_bad))"

# Every raw @route line in the tree must have become exactly one parsed tuple. This is exact and
# self-scaling: no floor to go stale, and a tag the parser cannot read is named rather than skipped.
raw_tags="$(grep -rhE '^///[[:space:]]*@route[[:space:]]' "$SRC_DIR" --include='*.rs' 2>/dev/null | wc -l || true)"
# Same idea on the router side: at least one registration per `.route(` seen in the source.
raw_routes="$(sed -n '/^fn api_routes/,/^}/p' "$APP_RS" | sed 's://.*$::' | grep -cF '.route(' || true)"

if [ "$raw_tags" -eq 0 ] || [ "$raw_routes" -eq 0 ]; then
	note "parsed NOTHING — $raw_tags raw @route tag(s), $raw_routes raw .route( registration(s)."
	echo "      A route-tag check with no inputs is not a pass. Either the crate moved or this"
	echo "      script's extractor is broken; both are red."
fi
if [ "$n_tags" -ne "$raw_tags" ]; then
	note "$raw_tags @route tag(s) in the tree but $n_tags parsed into (METHOD, PATH, HANDLER)."
	grep '^ORPHAN ' "$TMP/tags.txt" 2>/dev/null | sed 's/^/      orphan: /' || true
	echo "      A tag with no handler beneath it, or a malformed tag, is an unreadable claim."
fi
if [ "$n_routes" -lt "$raw_routes" ] || [ "$router_bad" -ne 0 ]; then
	note "$raw_routes .route( registration(s) in api_routes but only $n_routes parsed."
	grep '^UNPARSED ' "$TMP/router.txt" 2>/dev/null | sed 's/^/      /' || true
	echo "      A registration shape this extractor cannot read must not be silently skipped."
fi

# Sentinels: keyed, literal, present on BOTH sides. The last line of defence against a pipeline
# that produced two lists which are wrong in the same direction.
for s in "${SENTINELS[@]}"; do
	for side in router tags; do
		st="$(gate_probe_file -F "$s" "$TMP/$side.txt")"
		case "$st" in
		0) ;;
		1) note "sentinel absent from the $side extraction: '$s' — the parser lost a route that is known to be there." ;;
		*) _gate_tool_fail "sentinel probe on $side" "sentinel check" "$st" || fail=1 ;;
		esac
	done
done

if [ "$fail" -ne 0 ]; then
	echo
	echo "ROUTE-TAG CHECK: FAIL (the parse could not be trusted — no tag/route verdict was issued)"
	exit 1
fi

# ── The cross-check ──────────────────────────────────────────────────────────────────────────────
#
# Keys are wrapped in `|` on both sides so a literal (`-F`, substring) probe implies a whole-line
# match: `|GET /api/v1/servers list_servers|` cannot be a substring of any other key. This is what
# lets the comparison stay literal — and it must stay literal, because a route path's `{id}` is an
# invalid ERE repeat under ugrep (see the header).
awk '!/^UNPARSED /{print "|" $1 " " $2 " " $3 "|"}' "$TMP/router.txt" | sort -u > "$TMP/router.key"
awk '!/^ORPHAN /{print "|" $1 " " $2 " " $3 "|"}' "$TMP/tags.txt" | sort -u > "$TMP/tags.key"

probe_key() { # <key> <keyfile> <what>  -> 0 present, 1 absent, exits on tool failure
	local st
	st="$(gate_probe_file -F "$1" "$2")"
	case "$st" in
	0) return 0 ;;
	1) return 1 ;;
	*)
		_gate_tool_fail "$3" "route-tag cross-check" "$st"
		echo "      Refusing to issue a route-tag verdict from a comparison that did not run."
		exit 1
		;;
	esac
}

a_bad=0
echo "── A. @route tags with no matching route in $APP_RS ──"
while read -r m p fn loc; do
	[ "$m" = "ORPHAN" ] && continue
	probe_key "|$m $p $fn|" "$TMP/router.key" "tag→router" && continue
	echo "  $loc"
	echo "      @route $m $p  ->  handler \`$fn\` is NOT registered in $APP_RS on that method+path."
	a_bad=$((a_bad + 1))
done < "$TMP/tags.txt"
[ "$a_bad" -eq 0 ] && echo "  none — all $n_tags tag(s) resolve to a registered route."

b_bad=0
echo "── B. registered routes with no matching @route tag ──"
while read -r m p fn; do
	[ "$m" = "UNPARSED" ] && continue
	probe_key "|$m $p $fn|" "$TMP/tags.key" "router→tag" && continue
	echo "  $m $p"
	echo "      registered to \`$fn\`, which carries no matching @route tag (GO-7 requires one)."
	b_bad=$((b_bad + 1))
done < "$TMP/router.txt"
[ "$b_bad" -eq 0 ] && echo "  none — all $n_routes registered route(s) are documented."

echo
echo "checked $n_tags @route tag(s) against $n_routes registered route(s) in $APP_RS"
if [ "$a_bad" -ne 0 ] || [ "$b_bad" -ne 0 ]; then
	echo "ROUTE-TAG CHECK: FAIL — $a_bad unwired tag(s), $b_bad undocumented route(s)"
	echo "  A @route tag is a contract with the router, not a comment. Wire the route, move the"
	echo "  tag onto the handler that really serves it, or delete the claim."
	exit 1
fi
echo "ROUTE-TAG CHECK: PASS"
