# T-923 — Plan

## Context

`platform wave close` validates a wave and *prints* "WAVE {n} CLOSED" — a human then
types the marker commit. History shows humans mangle it: waves 231–235 got prefixed
subjects the anchored authority rejects as non-markers, and 218/233 needed disavow
reverts. That is why the ledger base sits at 132. The next close (133) is the first
post-T-914 ceremony; automating the marker before it happens deletes the whole error
class.

## Approach

In `xtask/src/wave/land.rs` (`cmd_wave_close`), after the existing validations pass:
refuse on a dirty working tree; build the subject `wave {n} CLOSED — {summary}`
(summary from `--summary`, newlines stripped); self-check it against
`wave_close_subject_ok` and the oracle (`wave_close_is_newest_wave` semantics) BEFORE
committing; write the marker commit (`--allow-empty`); run `wave repack` (T-914's
include-HEAD derivation exists exactly for this — base becomes {n}); auto-commit the
lock refresh as the follow-up commit, same shape as land's repack commit. `--dry-run`
prints the would-be subject and stops. The print-only behavior is replaced, not kept as
a default.

## Risks

Subject self-check and the committed authority could drift — mitigated by calling the
same `wave_close_subject_ok`/oracle functions, never a re-implementation. A crash
between marker commit and repack leaves check red with the recovery being exactly the
documented close→red→repack loop — acceptable, no new state. Summary text could smuggle
a delimiter that changes parsing — pinned by a test with hostile summaries.

## Verification

Fabricated-repo tests: close writes a marker the authority accepts with number {n};
repack derives base {n} and renumbers open waves {n}+1..; dirty tree refuses; hostile
summary still parses as wave {n}; `--dry-run` writes nothing (porcelain empty).
`cargo test -p xtask` green; `ticket check --strict` prints `check OK`;
`platform wave gate --slice T-923` prints `SLICE GATE: PASS`.
