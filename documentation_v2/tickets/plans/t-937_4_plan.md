# T-937.4 — Plan

## Context
persist.rs:827-832 swallows every save error into console.warn; :952-957 pagehide save is fire-and-forget;
:252/:461/:815-826 note_unreadable locks saving out until reload. Silent save loss is data loss (priority 0).

## Approach
1. Verify on main: force save_state_as Err → no observable change; paste the red.
2. `state/save_status.rs` (new, in state/mod.rs): SaveStatus signal, chip component, toast on Failed.
3. persist.rs: report every Err; name quota; visibilitychange-hidden flush; debounce ≤ 1 s.
4. note_unreadable: three retries with backoff, then lockout with a Retry action.
5. Perturbation: swallow the Err again → status test red; restore, touch, green.

## Risks
- Flush on hidden must not double-save with pagehide — one in-flight guard.
- Toast noise on transient failures — one toast per failure episode.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-937.4`
