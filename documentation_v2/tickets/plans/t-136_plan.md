# T-136 — Plan

## Context
aar_replay_url exists on matches (deployments.rs:83) but nothing produces a replay. T-940.13 adds the position-
bearing events; this ticket turns them into a timeline endpoint and a map scrubber.

## Approach
1. New `api/src/handlers/telemetry/replay.rs` (register in `telemetry/mod.rs`): query events for a match, bucket to
   1 Hz, page by `from_t`; tests on a fixture match (ordering, paging, empty match).
2. New `frontend/src/pages/public/aar_replay.rs` (register in `pages/public/mod.rs`): map canvas reuse, scrubber
   (play/pause/speed), unit dots with side colour; wasm tests for scrub state.
3. `pages/public/deployments.rs`: aar_replay_url → route to the page with the match id.
4. Perturbation: return events unsorted → ordering test red; restore, `touch`, green.

## Risks
- Event volume for long matches; 1 Hz bucketing plus paging keeps pages under 1 MB — measure and report.

## Verification
- `cargo test -p website-api replay` · leptos gates · `cargo xtask platform wave gate --slice T-136`
