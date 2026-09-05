# T-311 — Plan

## Context
leaderboards.rs:47-51 whitelists five ORDER BY arms with no secondary key; T-194's golden seeds 4-way ties, so
LIMIT/OFFSET paging can repeat and skip rows. Small fix, real correctness bug on a public page.

## Approach
1. `apps/website/api/src/handlers/telemetry/leaderboards.rs`: write the paging test first (golden seed, LIMIT 2 over
   the team_kills tie, collect pages, assert set equality and no duplicates) — red or flaky on main; paste it.
2. Append `, lt.discord_id ASC` to each of the five whitelist strings (:47-51).
3. Perturbation: remove it from `team_kills` → red; restore, `touch`, green.

## Risks
- discord_id nullable? Check the schema; fall back to `lt.user_id ASC` if it is.
- The test needs the DB test harness (`cargo xtask db up`); say so in the report if it was skipped — skip is FAIL.

## Verification
- `cargo test -p website-api leaderboards`
- `cargo xtask platform wave gate --slice T-311`
