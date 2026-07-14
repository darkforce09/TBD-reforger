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
  driver/     cdp.mjs serve.mjs freeze.js dom.js gate_{s,r,v,t}.mjs         ← pending
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

```bash
# Verify the driver against the real React oracle (no backend, no Leptos):
ORACLE_DIST=…/apps/website/frontend/dist node .ai/artifacts/t159_gates/driver/smoke.mjs
```

`smoke.mjs` drives `/login` (zero API calls) on two fresh loads and asserts the two normalized-DOM
captures are **byte-identical** — proving the serializer is deterministic, the property the V gate
rests on. Verified: identical DOM sha256 across runs; Aegis tokens resolve to computed values
(`color: rgb(221,226,247)`, `background: rgb(13,19,34)`), `#root` → positional `#0`.

## Status

- **Built + verified:** S-gate extractor + 5 React-oracle manifests (self-verifying); CDP driver
  (`cdp`/`serve`/`freeze`/`dom`) + `smoke.mjs` (deterministic DOM capture proven against real React).
- **Pending:** the R/V/T gate runners (`gate_{r,v,t}.mjs`), the fixture corpus (`fixtures/`), the
  Leptos-side `extract-leptos.mjs`, and the `t159-leptos` + `t159-gates` CI jobs — these come online
  as the Leptos app (T-159.1+) provides a target to gate against.
