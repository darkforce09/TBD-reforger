# T-159.1 — Leptos app scaffold (workspace member)

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree only:** `.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Problem

The suite UI is React/Vite. Operator chose a Leptos rewrite on a standing worktree. There is
no Leptos crate, no workspace membership, and no documented way to run a Rust UI next to the
existing Axum API.

## Locked decisions

Inherit hub L1–L8. Additionally:

| # | Decision |
|---|----------|
| S1 | Crate path: `apps/website-leptos/` (new workspace member). |
| S2 | Leptos **0.7+** (or current stable at implement time) with `csr` feature for the first spike; trunk or leptos-friendly Vite-alternative — pick one and document in crate README. |
| S3 | Dev: UI on a dedicated port (e.g. `:3000` or trunk default); API remains `make api` on `:8080`. CORS/proxy as needed for cookie/JWT. |
| S4 | Root `Cargo.toml` workspace `members` gains `apps/website-leptos`. |
| S5 | Do **not** remove or break `apps/website/frontend` React build. |
| S6 | Minimal UI: one route showing brand string **"TBD Reforger"** + "Leptos scaffold T-159.1" + link/button that `GET`s `/api/v1/health` or equivalent if present (else a documented placeholder). |
| S7 | Aegis: import or copy minimal CSS variables from existing `index.css` into the Leptos asset pipeline (enough to prove token reuse — not full page redesign). |

## Do

1. Add `apps/website-leptos` with Leptos CSR (or SSR if clearly simpler with Axum — default CSR).
2. Wire workspace + lockfile; `cargo check -p <crate>` green.
3. Document `README.md` in the crate: how to run UI + how to point at local API.
4. Optional Makefile target `make web-leptos` (or `make leptos-dev`) at repo root — nice-to-have.
5. Verify log at `.ai/artifacts/t159_1_verify_log.md`.
6. Commit + tag **T-159.1**.

## Do not

- Edit `docs/**` or `.ai/tickets/registry.json` (Cursor owns).
- Delete React frontend.
- Port Mission Creator / wgpu host yet (T-159.5+).
- Implement full auth (T-159.3).
- Nest another git worktree / `./scripts/ticket run`.

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
cargo check -p website-leptos   # or whatever package name you chose — record exact name
# React still builds:
cd apps/website/frontend && npm run build
```

Manual: open the Leptos app in a browser; see scaffold page.

## Claude Code prompt — T-159.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.1** — Leptos app scaffold (workspace member).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current   # expect t-159-leptos-ui
  # Do NOT checkout/create other branches; do NOT nest ./scripts/ticket run

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t159_1_claude_code_handoff.md
  2. docs/platform/t159_leptos_ui_program.md
  3. docs/platform/t159_1_leptos_scaffold.md
  4. Cargo.toml (workspace members)
  5. apps/website/Cargo.toml (Axum API — do not gut)
  6. apps/website/frontend/src/index.css (Aegis tokens to reuse minimally)

═══ PROBLEM ═══
  No Leptos UI crate yet. Need a workspace member apps/website-leptos that runs a minimal
  CSR (default) app next to the existing React SPA and Axum API, proving the rewrite lane.

═══ SHIPPED (do not reopen) ═══
  T-159.0 program hub + standing worktree (Cursor docs).
  T-145 Axum backend + yrs wasm; T-151 wgpu map engine — leave alone this slice.

═══ LOCKED ═══
  - Crate: apps/website-leptos ; add to workspace members
  - Leptos 0.7+; CSR OK for .1; document run command
  - UI dedicated port; API :8080 unchanged
  - Minimal page: brand TBD Reforger + scaffold label + optional health fetch
  - Copy/import minimal Aegis CSS variables — not a redesign
  - React frontend must still npm run build
  - No auth, no MC, no registry edits, no docs edits

═══ DO ═══
  1. Scaffold apps/website-leptos with Leptos; wire Cargo workspace + lockfile
  2. One route UI as locked; crate README with run instructions
  3. Optional make web-leptos / leptos-dev
  4. Write .ai/artifacts/t159_1_verify_log.md (toplevel path, HEAD SHA, cargo check, npm build)
  5. Commit prefix T-159.1: · tag T-159.1
     Co-Authored-By: Claude Code <noreply@anthropic.com>

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, CLAUDE.md ticket-sync markers
  - Remove or break apps/website/frontend
  - Port Mission Creator / map wasm host
  - git checkout -b / nested worktrees / ./scripts/ticket run

═══ VERIFY (all exit 0) ═══
  cargo check -p <leptos-package-name>
  cd apps/website/frontend && npm run build
  Record package name + ports in verify log.

═══ RETURN ═══
  - Commit SHA + tag T-159.1
  - .ai/artifacts/t159_1_verify_log.md
  - Ready for Cursor doc sync.
```
