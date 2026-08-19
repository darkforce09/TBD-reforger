# Editor wave 211 — briefs staged

Tickets: **T-843**, **T-842**, **T-826** (historical band 211 — not lock wave 137 whole).

Status: **READY TO DISPATCH** — waiting for operator word (`dispatch` / `run 211`).

Stack (this prep):
- Postgres `:5434` up
- API `:8080` healthz 200
- Leptos/trunk release `:3000` (started with `unset NO_COLOR` — trunk 0.21.14 rejects `NO_COLOR=1`)

Worktrees:
- `.ai/artifacts/worktrees/T-843` → `slice/T-843`
- `.ai/artifacts/worktrees/T-842` → `slice/T-842`
- `.ai/artifacts/worktrees/T-826` → `slice/T-826`

W210 hold-list filed (not in this wave): T-926 Transform, T-927 dblclick leak, T-930 first-paint disc; T-931 seat-zoom **deferred**.

Note: prep ticket/lock edits are **uncommitted on main** — commit before close/dispatch if you want worktrees to inherit owns via git (T-843.toml was copied into the T-843 worktree for the owns retarget).
