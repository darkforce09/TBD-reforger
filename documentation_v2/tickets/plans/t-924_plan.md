# T-924 — Plan

## Context
A gate that silently refused (2026-08-14) did not stop a land; the verdict lives only in the terminal. T-913.2's
token receipt is the proven shape: refuse without a fresh receipt, forward-only, no backfill.

## Approach
1. New `xtask/src/wave/verdict.rs` (register in `wave/mod.rs`): `Verdict {sha, verdict, at}`, `write(slice)`,
   `read(slice)`, path `.ai/artifacts/verdicts/<slice>.json`; tests: round-trip, stale-sha detection.
2. `wave/gate.rs` cmd_gate/gate_slice: write the receipt with the slice HEAD after the verdict is known.
3. `wave/land.rs` cmd_land: read the receipt; refuse (missing / not green / sha ≠ landing HEAD) with the exact
   `cargo xtask platform wave gate --slice T-xxx` fix line.
4. Perturbation: land ignores the sha comparison → refusal test red; restore, `touch`, green.

## Risks
- Rebased slices change HEAD after a gate; that is the intended refusal — the fix line re-gates.

## Verification
- `cargo test -p xtask wave::verdict wave::land` · `cargo xtask platform wave gate --slice T-924`
