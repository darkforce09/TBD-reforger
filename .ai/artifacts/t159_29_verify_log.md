# T-159.29 — cutover build-out — verify log

**Slice:** T-159.29 (finish program stream 6 — the cutover **build-out**; the actual default flip +
React deletion are HELD for operator).
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159` · branch `t-159-leptos-ui` · **base** `61327661` (T-159.28).
**Executor:** claude-code (solo session). **Result: PASS.**

## Goal

Build the infrastructure to serve the Leptos SPA in production and keep it green in CI — everything
the flip needs, without the destructive flip itself.

## What shipped

- **Backend static SPA serve** (`apps/website/src/{config,app}.rs`) — new `SPA_DIST_DIR` +
  `MAP_ASSETS_DIR` config. When `SPA_DIST_DIR` is set, `router()` serves the Leptos dist statically
  with **COOP `same-origin` + COEP `credentialless`** (the wasm SharedArrayBuffer headers, mirroring
  Vite/Trunk), an **SPA fallback** to `index.html` for client routes, and `/map-assets` from
  `MAP_ASSETS_DIR` (default `../../packages/map-assets`) — so the app runs **same-origin with the
  API** (no separate web host). Unset in dev (Trunk owns the SPA); the API stays API-only.
- **CI job** (`.github/workflows/ci.yml`) — a `website-leptos` job: `cargo fmt --check` + `clippy`
  (wasm32) + `cargo test` (native shell) + `trunk build --release`. The G gate that keeps the branch
  compilable in CI. (The editor S/V/R/T smokes need a cached headless chromium — the T-151 pattern —
  and run as a follow-on once the SPA is default.)
- **Leptos crate fmt** — `cargo fmt -p website-leptos` (19 files) so the new CI `fmt --check` is
  green (prior slices left known drift out-of-scope). Formatting only; **behavior-verified** (5
  editor smokes re-run green post-fmt).
- **env docs** (`apps/website/.env.example`) — `SPA_DIST_DIR` / `MAP_ASSETS_DIR` documented with the
  flip note: at cutover, point `FRONTEND_URL` / `ALLOWED_ORIGINS` / the Discord app redirect URI at
  the API's own origin instead of `:5173`.
- **make targets** — `leptos` / `leptos-build` / `leptos-gates` already shipped in T-159.24.
- **Oracle freeze** — the S-manifest comparators (`routes.csv` 26 / `hooks.csv` 48 /
  `components.csv` 40 / `css_tokens.txt` 137 / `deps.csv` 26) are committed and drift-free
  (`extract-react.mjs --check` exit 0), so the structural oracle is preserved for the delete step.
  The V DOM/PNG capture freeze is part of the operator-gated delete (below).

## Live verification (SPA serve, throwaway API on :8099 with SPA_DIST_DIR set)

| Check | Result |
|-------|--------|
| `GET /` | **200 text/html** (index.html) |
| `GET /missions/abc/edit` (deep link) | **200 text/html** (SPA fallback) |
| COOP / COEP headers on `/` | `cross-origin-opener-policy: same-origin` · `cross-origin-embedder-policy: credentialless` |
| `GET /<hash>_bg.wasm` | **200 application/wasm** |
| `GET /healthz` | **200** (API intact) |
| `GET /api/v1/leaderboards` (no auth) | **401** (API auth still gated) |
| `GET /map-assets/everon/manifest.json` | **200** |

## Gates

| Gate | Result |
|------|--------|
| `cargo check -p reforger-backend` | clean (SPA serve additive) |
| `cargo check -p website-leptos --target wasm32-unknown-unknown` | clean |
| `cargo fmt -p website-leptos --check` | **clean** (the new CI job's fmt gate is green) |
| `cargo clippy … wasm32` | 12 = baseline |
| `trunk build --release` | ✅ success |
| 5 editor smokes (post-fmt behavior check) | 5/5 PASS |

## HELD for operator (destructive — not done)

- **The default flip** — set `SPA_DIST_DIR` in the prod `.env`, flip `FRONTEND_URL` /
  `ALLOWED_ORIGINS` / Discord redirect, staging soak. Needs real OAuth + a real 142 MB save over a
  real network + a rollback artifact (the last React dist).
- **React deletion** — delete `apps/website/frontend/`, purge the npm CI job, re-home the 16 `_wasm`
  parity oracles → `cargo test`, and the **V DOM/PNG oracle freeze** (capture the React DOM for all
  26 routes as goldens before the comparator is deleted). These are the `.29`-delete phase, gated on
  operator go per the finish plan.

## Next

**T-159.27** — Arsenal + registry compat + Faction Manager (the last large feature port; fills the
Attributes Arsenal stub). After it, the program's remaining work is the operator-gated flip + delete.
