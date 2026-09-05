# T-944 — Plan

## Context
T-940.6 shipped the NOTIFY-driven audit stream and disclosed two gaps it could not close inside its
contract: the `id > last_id` watermark drops a row whose transaction commits after a higher id was
streamed, and a half-open socket keeps the listener "up" so the poll fallback never engages.

## Approach
1. Stream by NOTIFY payload id with a dedupe ring; on Resync reconcile by committed_at window.
2. Listener heartbeat with deadline → Down → redial; ticker polls while Down.
3. Two integration tests reproducing the race and the half-open loss; perturb each fix.

## Risks
Dedupe ring size vs burst rate; heartbeat cost on the listener connection (keep it ≥ 5 s).

## Verification
`TBD_IT_BASE_DB=tbd_slice_t944_it cargo xtask db test-it`; `cargo xtask platform wave gate --slice T-944`.
