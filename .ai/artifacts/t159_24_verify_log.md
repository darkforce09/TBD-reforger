# T-159.24 — prep: dev API proxy + make targets + client verbs + 140 MB upload spike — verify log

**Slice:** T-159.24 (first stream of the single-session finish program — audit + plan
`~/.claude/plans/you-are-fable-5-vast-bird.md`, operator-approved 2026-07-17).
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159` · branch `t-159-leptos-ui` · **base** `997da9e5`
(T-159.22.1). **Executor:** claude-code (solo session — code + docs + commits, no Cursor pass).
**Result: PASS** — all gates green, spike decisive.

## What shipped

- **`apps/website-leptos/Trunk.toml`** — two `[[proxy]]` blocks (Trunk 0.21.14): `/api` →
  `http://127.0.0.1:8080/api` (the Vite `/api → :8080` dev equivalent — relative `/api/v1/*` calls
  now reach the backend under `trunk serve`), and `/map-assets` → backend (dead 404 until the
  T-159.29 flip build-out serves it; declared now so the dev origin story is fixed at same-origin
  absolute `/map-assets/*`).
- **`Makefile`** — `leptos` (trunk serve :3000), `leptos-build` (release dist), `leptos-gates`
  (release build + every `*_editor.mjs` smoke); `.PHONY` updated.
- **`apps/website-leptos/src/client.rs`** — the two hand-rolled `api_get`/`api_post` bodies
  collapsed into one `request()` (method + optional JSON body + `Consume::Json|Ignore`) so the
  api/client.ts contract (bearer inject + single-flight 401 refresh + exactly one retry) can never
  diverge per-verb; new public verbs for the T-159.25 suite live-wire: **`api_put`**, **`api_patch`**
  (JSON-in/JSON-out), **`api_delete`**, **`api_post_ok`** (status-only — 204s / discarded bodies,
  axios parity). `#[allow(dead_code)]`'d until .25 wires them; `api_get`/`api_post` signatures
  unchanged (all existing call sites untouched).
- **`.ai/artifacts/t159_gates/driver/spike_upload_140mb.mjs`** — evidence script (not a suite gate).

## The 140 MB upload spike — transport risk retired

The React app needed a direct-`:8080` bypass because the **Vite proxy** reset large bodies
(T-060.1.4). Question: does the Leptos dev path (browser `fetch` — the same primitive gloo-net
wraps — through the new **Trunk** proxy) move a mission-version-scale body?

Method: real chromium (CDP) on the `trunk serve` origin, `fetch` POST of a **140,000,099-byte**
JSON body to `/api/v1/missions/<nil-uuid>/versions` with a real dev-login Bearer. Axum's `Json`
extractor reads + parses the full body before the handler can look up the mission (route body cap
256 MB), so a 4xx **response** proves every byte crossed; a socket reset would surface as a fetch
`TypeError` (what React saw at ~5.5 MB pre-T-060.1.4).

```json
{"gate":"spike-upload-140mb","ok":true,"status":404,"ms":940,
 "bodyBytes":140000099,"responseHead":"{\"error\":\"mission not found\"}","pass":true}
```

**940 ms, full body parsed, clean 404.** Conclusions: (1) no direct-`:8080` bypass port is needed
for the Leptos editor's Save-at-scale path in dev; (2) post-flip (same-origin serve) has no proxy at
all; (3) gloo-net inherits this result (same `fetch`, string body). Proxy passthrough also verified
cheap: `GET :3000/api/v1/leaderboards` → backend `401` (auth required — reached the API).

## Plan deviations (recorded, deliberate)

The approved plan listed two more prep items designed for **concurrent multi-session** execution:
the bulk DTO append from `hooks.csv` and pre-registered module stubs/seams (`main.rs` mods,
`editor_ops` dialog seam, `world_assets::attach`). Execution is **serial single-writer** in this
session, so the conflict-avoidance rationale is void; DTOs and module registrations land with the
stream that uses them (.25/.26/.27/.28) where they are testable, instead of as dead code here.

## Gates (all green)

| Gate | Result |
|------|--------|
| `cargo check -p website-leptos --target wasm32-unknown-unknown` | clean (0 warnings) |
| `cargo check -p website-leptos` (native) | **9** warnings — **stash-diff byte-identical to base** (`git stash -u` → 9 → pop → 9); pre-existing dead-code set, toolchain drift vs the "8" in older logs |
| `cargo clippy -p website-leptos --target wasm32-unknown-unknown` | **12** warnings — **stash-diff identical to base** (12 before, 12 after); zero new lints, no `client.rs` hits |
| `trunk build --release` | ✅ success |
| **11 editor smokes** (baseline @ `997da9e5` before edits) | **11/11 PASS** |
| **11 editor smokes** (after edits, fresh release dist) | **11/11 PASS** |
| Native unit tests (`cargo test -p website-leptos` client contract) | unchanged (send_with_refresh untouched; 3 tests still cover single-retry/no-loop/non-401) |

Smoke table (post-change run, fresh `trunk build --release` dist): selfcheck_editor ·
smoke_editor · smoke_pan_editor · smoke_doc_editor · smoke_persist_editor · smoke_select_editor ·
smoke_marquee_drag_editor · smoke_save_export_editor · smoke_undo_editor · smoke_cur_editor ·
smoke_outliner_palette_editor — **all PASS, exit 0**.

## Ops notes

- `apps/website/.env` is untracked and absent in a fresh worktree checkout — copied from the main
  repo (`cp ../../..../apps/website/.env apps/website/.env`) so `cargo run --bin api` boots.
  `FRONTEND_URL`/`ALLOWED_ORIGINS` still point at `:5173` — flip documented at T-159.29.
- Stack for live gates: `make db-up` (podman `tbd_reforger_db` :5434) + `cargo run --bin api`
  (migrates on boot) + dev-login 302 mints a real session (proven).

## Next

**T-159.25** — suite live-wire (toast primitive → 23 mutations with live dev-login proofs → 6 mock
pages live → SSE ×2 → populated content branches → CreateMissionDialog).
