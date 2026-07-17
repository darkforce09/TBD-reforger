# T-165.10 verify log — closure: the last two ports + the no-node hard gate

## Ported

- **`xtask verify file-length`** (verify-file-length.mjs): SIZE-1/3 walk over
  `apps/website` .go/.ts/.tsx with the hand-rolled `.coding-standards-allowlist.yaml`
  parse — **stdout byte-identical** to the Node gate (`file-length: 0 warning(s),
  0 violation(s).`), rc 0/0. Makefile `verify-coding-standards` step flipped.
- **`xtask gen font-table`** (gen-text-font-table.mjs, 30-min-rule → ported): Spleen
  16×32 BDF → `text_font_table.rs` on stdout with the eyeball-proof glyphs on stderr;
  the committed table's GENERATED header and the `map-engine-render/src/lib.rs` pointer
  now cite the cargo command.

## The closure gate — `xtask verify no-node` (`make verify-no-node`)

Three checks, verify-no-python pattern:
1. `git ls-files '*.mjs' '*.cjs'` outside `apps/mod` → **0** (was 90 at T-165.0).
2. No `node `/`npx ` command-position invocations in the Makefile, `scripts/`, or
   `.github/` outside the enfusion-mcp floor (allowlist: `scripts/mod/mcp-call.sh`,
   whose `.js`-entry runner tiers are the floor by design). One straggler found and
   fixed: `manual-test.sh`'s `node -e` JSON-validity probe → `jq` (guarded).
3. Zero `actions/setup-node` in workflows (already true — the T-159/T-164 cleanups
   removed the last one).

Gate self-test en route: the scanner's first heuristic flagged its own Makefile help
text (`## … node only as the enfusion-mcp floor`) — tightened to command-position
matches with inline `##` doc text stripped. `make verify-no-node` rc=0; wired into
`ci-local` beside `verify-no-python`.

## Doc sync

CLAUDE.md frontend/toolchain line (Node 26 line → "All tooling is Rust; Node exists
solely as the enfusion-mcp runtime"), DEV_RUNBOOK toolchain rows + `nvm use` step,
Makefile ci-local comment, deploy-staging.sh header. Hub status → **COMPLETE**;
registry T-165 → `shipped` (active_slice cleared).

## Program end-state (T-165.0 → .10)

- **90 tracked .mjs (13.8k LOC) + dom.js/freeze.js + bcdec.wasm → 0** (apps/mod excluded
  by charter). Survivor: the third-party `enfusion-mcp` npm package under `scripts/mod`
  (+ its node_modules), the declared floor.
- packages/tbd-schema is npm-free (package.json/lock/node_modules deleted @ .9).
- Makefile: zero node/npx lines; every gate/builder is `cargo run -p xtask|tbd-tools`.
- CI (`ci.yml`/`contracts.yml`/`schema.yml`): zero setup-node, zero node steps — green
  at every slice tag from .0 through .9.
- Frozen contracts held throughout: V-suite 25/25 · editor smokes 16/16 · mcp selftest
  19/19 · census 1623/1,216,109/315/888/36/625 · schema gates 12/12 · `ticket check`.

## Gate suite at close

`make verify-no-node` rc=0 · `make verify-no-python` (inside ci-local) ·
`cargo clippy -p xtask -p tbd-tools --all-targets -- -D warnings` 0 · fmt clean ·
`./scripts/ticket check` OK · full `make ci-local` green (see the run capture in
`scratchpad/ci-local-final.txt` mirrored below the commit) · CI green on push.
