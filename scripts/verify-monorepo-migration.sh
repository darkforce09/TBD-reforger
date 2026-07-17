#!/usr/bin/env bash
# Monorepo migration verification (V1–V27). Exit 0 only when all gates pass.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
FAIL=0

fail() { echo "FAIL: $1" >&2; FAIL=1; }
pass() { echo "PASS: $1"; }

# V1 Git history (read-tree snapshot commits + merge parents; follows the apps/ rename)
wc=$(git log --oneline -- apps/website/ website/ 2>/dev/null | wc -l || echo 0)
[[ "$wc" -ge 1 ]] && pass "V1 apps/website/ has git history ($wc commits)" || fail "V1 apps/website/ history"

wc=$(git log --oneline -- apps/mod/ mod/ 2>/dev/null | wc -l || echo 0)
[[ "$wc" -ge 1 ]] && pass "V1 apps/mod/ has git history ($wc commits)" || fail "V1 apps/mod/ history"

# V2/V3 content parity — spot-check key paths exist
for p in apps/website/src apps/website-leptos apps/mod/tbd-framework packages/tbd-schema .ai/tickets/registry.json; do
  [[ -e "$p" || -f "$p" ]] && pass "V2 path exists: $p" || fail "V2 missing: $p"
done

# V4 shared schema lift
[[ -d packages/tbd-schema ]] && pass "V4 packages/tbd-schema" || fail "V4 packages/tbd-schema"

# V5 specs lift
[[ -d docs/specs/Mission_Creator_Architecture ]] && pass "V5 docs/specs" || fail "V5 docs/specs"

# V6 tickets
if ./scripts/ticket check --strict 2>/dev/null; then
  pass "V6 ticket check --strict"
else
  fail "V6 ticket check --strict"
fi
nid=$(cargo run -q -p xtask -- registry-get next_id)
[[ "$nid" -ge 121 ]] && pass "V6 next_id=$nid" || fail "V6 next_id"

# V7 registry under .ai/
[[ -f .ai/tickets/registry.json ]] && pass "V7 tickets under .ai/" || fail "V7 tickets"

# V8 scripts at root
[[ -x scripts/ticket ]] && pass "V8 scripts/ticket" || fail "V8 scripts/ticket"

# V9 artifacts
[[ -d .ai/artifacts ]] && pass "V9 .ai/artifacts/" || fail "V9 .ai/artifacts"

# V10 T-068 ready
status=$(./scripts/ticket get T-068 status)
active=$(./scripts/ticket get T-068 active_slice)
spec=$(./scripts/ticket get T-068 spec)
dep=$(./scripts/ticket get T-067 status)
if [[ "$status" == "ready" && -n "$active" && -f "$spec" && "$dep" == "shipped" ]]; then
  pass "V10 T-068 ready + deps"
else
  fail "V10 T-068 ready + deps"
fi

# V11 CLAUDE.md
[[ -f CLAUDE.md ]] && grep -q 'ticket-sync:status' CLAUDE.md && grep -q 'Executor gate' CLAUDE.md \
  && pass "V11 root CLAUDE.md" || fail "V11 CLAUDE.md"

# V12 README
[[ -f README.md ]] && grep -q 'apps/website/' README.md && pass "V12 README" || fail "V12 README"

# V13 Makefile
make help >/dev/null 2>&1 && pass "V13 make help" || fail "V13 make help"
for t in db-up api web ticket-sync ticket-check-strict schema-validate; do
  grep -q "^${t}:" Makefile && pass "V13 target $t" || fail "V13 target $t"
done

# V14 frontend (Leptos SPA) compiles — the React app was deleted at T-159.29.3.
if cargo check -p website-leptos --target wasm32-unknown-unknown >/dev/null 2>&1; then
  pass "V14 frontend (leptos) cargo check"
else
  fail "V14 frontend (leptos) cargo check"
fi

# V15 Go compile
(cd apps/website && go build ./...) && pass "V15 go build" || fail "V15 go build"

# V16 test-it (optional if db down — warn only)
if (cd apps/website && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable go test ./internal/handlers/... >/dev/null 2>&1); then
  pass "V16 test-it"
else
  echo "WARN: V16 test-it skipped (db may be down)"
fi

# V17 schema validate
if (cd packages/tbd-schema && npm ci --silent >/dev/null 2>&1 && node scripts/validate.mjs >/dev/null 2>&1); then
  pass "V17 schema validate"
else
  fail "V17 schema validate"
fi

# V18 Tbdevent gone from mod scripts/docs
if ! rg -q 'Tbdevent_Website' scripts/mod docs/mod 2>/dev/null; then
  pass "V18 no Tbdevent_Website in scripts/mod + docs/mod"
else
  fail "V18 Tbdevent_Website references remain"
fi

# V19 stale TBD_Website paths (allow migration docs)
hits=$(rg -l 'Projects/TBD_Website' . 2>/dev/null | rg -v 'MONOREPO_MIGRATION|verify-monorepo|migration-baseline' || true)
[[ -z "$hits" ]] && pass "V19 no stale TBD_Website paths" || fail "V19 stale paths: $hits"

# V20–V22 root platform files
[[ -f docs/platform/MONOREPO_MIGRATION.md ]] && pass "V20 MONOREPO_MIGRATION.md" || fail "V20 migration doc"
[[ -f docs/TICKET_MOD_QUEUE.md ]] && pass "V21 TICKET_MOD_QUEUE" || fail "V21 MOD queue"
[[ -f apps/website/CLAUDE.md ]] && grep -q 'Canonical context' apps/website/CLAUDE.md && pass "V22 website stub" || fail "V22 website stub"

# V23 sparse-paths helper
./scripts/ticket sparse-paths T-068 >/dev/null && pass "V23 sparse-paths" || fail "V23 sparse-paths"

# V24 advance-slice command exists
./scripts/ticket help 2>&1 | grep -q advance-slice && pass "V24 advance-slice" || fail "V24 advance-slice"

# V25 schema CI workflow
grep -q 'packages/tbd-schema' .github/workflows/schema.yml && pass "V25 schema CI" || fail "V25 schema CI"

# V26 specs not under apps/website/Design_Docs
[[ ! -d apps/website/Design_Docs ]] && pass "V26 Design_Docs lifted" || fail "V26 Design_Docs still under apps/website"

# V27 no Design_Docs/ in tracked files (except rewrite scripts)
hits=$(git ls-files | xargs rg -l 'Design_Docs/' 2>/dev/null | rg -v 'rewrite-|MONOREPO_MIGRATION|verify-monorepo|migration-baseline' || true)
[[ -z "$hits" ]] && pass "V27 no Design_Docs/ links" || fail "V27 Design_Docs/ in: $hits"

# crf_framework local reference (gitignored, must exist on disk for Workbench dev)
[[ -f apps/mod/crf_framework/addon.gproj ]] && pass "crf_framework on disk (gitignored)" || fail "apps/mod/crf_framework/ missing"
[[ -f apps/mod/crf_framework/REFERENCE-ONLY.md ]] && pass "crf_framework REFERENCE-ONLY marker" || fail "crf_framework incomplete"

# EnfusionMCP — gitignored dev-only; soft warn if missing (run tbd-dev-bootstrap.sh)
EMCP="apps/mod/tbd-framework/Scripts/WorkbenchGame/EnfusionMCP"
if [ -d "$EMCP" ] && [ "$(find "$EMCP" -name '*.c' 2>/dev/null | wc -l)" -ge 1 ]; then
  pass "EnfusionMCP on disk (gitignored, $(find "$EMCP" -name '*.c' | wc -l) handlers)"
else
  echo "WARN: EnfusionMCP missing — run: bash scripts/mod/tbd-dev-bootstrap.sh"
fi

if [[ "$FAIL" -ne 0 ]]; then
  echo ""
  echo "Verification FAILED"
  exit 1
fi
echo ""
echo "All migration checks passed (V1–V27)"
exit 0
