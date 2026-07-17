# T-159 gate harness

Machine-checked parity gates for the React → Leptos migration (T-159). **Identity = gates, not
vibes.** A slice ships only when every applicable gate exits 0 against the React oracle.

Substrate: the repo's existing zero-npm-dep CDP driver (`scripts/website/verify-wgpu-gpu.mjs`),
generalized here. One driver, five gate verbs, one committed fixture corpus.

## The five gates

| Gate | Proves | Mechanism |
|------|--------|-----------|
| **G** | Build/green | `cargo check` + `clippy -D warnings` + `trunk build` (wasm32); React still builds until cutover |
| **S** | Structural inventory | A manifest extracted from the Leptos source **set/column-diffs equal** to the same manifest from React |
| **R** | Response/state equality | Canonical-JSON byte-equal: API DTO vs golden fixture; yrs `encode_state` after a mutator script; `tbd-auth` persist blob |
| **V** | Visual identity | **Primary:** normalized DOM + computed-style byte-equal (deterministic). **Secondary:** pixel ε=0 at 1440×900 (small ε for AA/blur only). **Map:** GPU-readback self-checks |
| **T** | Interaction trace | Scripted pointer/keyboard → equal state hash; includes the single-flight-refresh proof (two concurrent 401s → exactly one `/auth/refresh`) |

## Layout

```
t159_gates/
  manifests/  extract-react.mjs   ← S-gate: React-oracle extractor (BUILT)
              extract-leptos.mjs   ← per-slice: same schemas from Leptos source (pending)
              routes.csv hooks.csv components.csv css_tokens.txt deps.csv   ← committed goldens
  driver/     (T-165.6: retired — the harness is Rust: `tools/tbd-tools` `gate` bin;
              `gate editor-suite` + `gate v-suite verify` + `gate s-routes` + `gate r-auth`
              + `gate render-check` + `gate smoke <name>`; freeze/dom payloads live as
              verbatim consts in tools/tbd-tools/src/inject.rs)
  fixtures/   api/ doc/ trace/ auth/                                        ← pending
  v/<slice>/  <route>.{dom.json,oracle.png,leptos.png,diff.png}            ← run artifacts
  logs/<slice>/gate_table.json     ← per-slice verify log (the R/S/V/T/G pass rows)
```

## S gate — structural manifests (built)

`manifests/extract-react.mjs` produces five deterministic (sorted) manifests from the React source.
Zero npm deps (Node built-ins) so it runs in CI without `npm ci`.

```bash
node .ai/artifacts/t159_gates/manifests/extract-react.mjs           # (re)write manifests
node .ai/artifacts/t159_gates/manifests/extract-react.mjs --check   # drift check; exit 1 on drift
```

Measured baseline (ground truth — supersedes the inventory's estimates):

| Manifest | Count | Schema | Notes |
|----------|------:|--------|-------|
| `routes.csv` | 26 | `path,component,fullBleed,chromeless,router_auth` | leaf routes; AppLayout container + DEV `/_spike/*` excluded |
| `hooks.csv` | 48 | `name,kind,method,url` | 24 query + 24 mutation route-tags (23 mutation fns; `useSaveFaction` = POST+PUT) |
| `components.csv` | 40 | `name,kind,path` | 27 ui + 4 layout + 9 shell — per exported identifier, incl. `export { … }` blocks |
| `css_tokens.txt` | 137 | one `--token` per line | unique custom-property names (`--radius-md` declared twice → 1) |
| `deps.csv` | 26 | `npm_pkg,disposition,replacement` | disposition ledger; a new runtime dep with no disposition fails the extractor |

The S gate (`gate_s.mjs`, pending) runs both extractors and asserts the two CSVs are row-set and
column equal. Until the Leptos side exists, `--check` self-verifies the React oracle is reproducible.

## CDP driver (built + verified)

`driver/` — the reusable, zero-dep CDP client (`cdp.mjs`), the determinism payload (`freeze.js`),
the V-primary normalized-DOM serializer (`dom.js`), and a static SPA server (`serve.mjs`).

**T-159.29.3 — the React app is deleted.** The live-oracle era is over; the V gate is now the
frozen suite: `node driver/gate_v_suite.mjs verify` diffs the Leptos dist against the committed
`v/oracle-freeze/` goldens (25 routes, captured from the final React dist at T-159.29.1 — see
`v/oracle-freeze/manifest.json` for per-route sha256 + the oracle dist identity). `gate_v.mjs`
(single-route, live-oracle) and the deleted `smoke.mjs` (serializer determinism proof, verified
against real React before the freeze) are historical. `manifests/extract-react.mjs` can no longer
run — the five S-manifests are frozen goldens of the final React source.

## Status

- **Frozen (T-159.29.1/.3):** 5 S-manifests + 25 V DOM goldens (`v/oracle-freeze/`) — the React
  oracle's final state; the app itself is deleted. Permanent gates: `gate_v_suite.mjs verify`,
  the 15 `smoke_*_editor.mjs` CDP smokes, `gate_r_auth.mjs`, and the `dto.rs` R-api cargo tests
  (all wired into `make leptos-gates`).
