# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.16** (`WasmMissionDoc` host) · **Latest MC map:**
**T-159.15.2** @ `ebcabe1d` (tag **T-159.15.2**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` · branch `t-159-leptos-ui` · **Authority:** this hub ·
[`.ai/tickets/registry.json`](../../.ai/tickets/registry.json)

## In one sentence

Rework the website SPA from React/Vite into **Leptos (Rust)**, sharing types with the
existing Axum backend and hosting the existing map/mission wasm engines — developed on a
**standing worktree**, merge to `main` when cutover-ready.

## Execution model (worktree-only)

Same discipline as **T-151**: CWD = worktree; linear commits on `t-159-leptos-ui`; tags
`T-159.n`; no nested `./scripts/ticket run`; React stays until cutover.

## Locked decisions

| # | Decision |
|---|----------|
| L1 | **Leptos** UI framework |
| L2 | Crate `apps/website-leptos` — do not replace Axum |
| L3 | API on `:8080` |
| L4 | Aegis tokens/CSS |
| L5 | Map engine + mission doc cores stay Rust; Leptos hosts |
| L6 | React buildable until cutover |
| L7 | Shared Rust API types |
| L8 | T-068 on `main` parallel OK |
| L9 | Pages = oracle DOM; map = GPU/smoke Class R |
| L10 | Dates via `js_sys::Date` + freeze.js |
| L11 | No `RenderEngine::unproject_xy` (X-05); pan via `engine.pan` |

## Progress (worktree tip `ebcabe1d`)

| Milestone | Status |
|-----------|--------|
| 24 page routes byte-identical | shipped |
| **T-159.15.0** wgpu boundary collapse | `3066f14c` |
| **T-159.15.1** render loop + wheel + GPU gate | `a425936d` |
| **T-159.15.2** MMB/RMB pan + mid-pan wheel rebase | `ebcabe1d` tag **T-159.15.2** |
| **T-159.16** MissionDoc host | **ACTIVE** |
| .17–.22 Eden shell / cutover | queued |

### T-159.15.1 root cause

[`.ai/artifacts/t159_15_1_verify_log.md`](../../.ai/artifacts/t159_15_1_verify_log.md) — Dawn **GpuTimer**
double-map → `disable_frame_timing()`; `poll()` kept. Follow-up **T-160**.

### T-159.15.2

[`.ai/artifacts/t159_15_2_verify_log.md`](../../.ai/artifacts/t159_15_2_verify_log.md) — incremental
`engine.pan`; smoke math Class R (7200 after −200px @ z=−2; rebase by construction).

## Slice index

| Slice | Goal | Status |
|-------|------|--------|
| **T-159.0–.14** | Scaffold → pages | shipped on branch |
| **T-159.15.0** | Boundary collapse | shipped `3066f14c` |
| **T-159.15.1** | Loop + wheel | shipped `a425936d` |
| **T-159.15.2** | Pan + rebase | shipped `ebcabe1d` |
| **T-159.16** | MissionDoc host | **ready** — `t159_16_mission_doc_host.md` |
| **T-159.17–.22** | Persist → tools → save → outliner → Arsenal | queued |
| **T-159.23–.25** | Sweep → cutover → delete React | queued |

Inventory: [`.ai/artifacts/t159_leptos_full_migration_inventory.md`](../../.ai/artifacts/t159_leptos_full_migration_inventory.md)

## Ops

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
# smokes: .ai/artifacts/t159_gates/driver/smoke_editor.mjs | selfcheck_editor.mjs | smoke_pan_editor.mjs
```
