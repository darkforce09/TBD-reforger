# T-940.2 — Plan

## Context
events.rs:2037-2041 hard-deletes on withdraw; models/event.rs:56 no_show has no writer; unlinked players leave no
attendance. After T-940.1 (events.rs). core/dto.rs is the R-api golden mirror.

## Approach
1. Verify on main: withdraw then list shows no row; paste the red.
2. `migrations/0023_registration_tombstones.sql`: withdrawn_at; unlinked_attendance(event_mission_id, arma_id).
3. Withdraw → state withdrawn + withdrawn_at; seat index ignores withdrawn rows.
4. Results path: absent registered players → no_show; unlinked result players → unlinked_attendance, reconciled on link.
5. Mirror models/event.rs and core/dto.rs.
6. Perturbation: writer skips absent players → no_show test red; restore, touch, green.

## Risks
- Reconciliation on link must be idempotent.

## Verification
- `cargo xtask db test-it`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-940.2`
