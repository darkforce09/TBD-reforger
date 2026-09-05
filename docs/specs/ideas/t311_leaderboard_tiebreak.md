# T-311 — Leaderboard ORDER BY lacks a tie-breaker

Owner: command center. Frozen-scope ticket; proposed scope website/backend/http_api.

## Claude Code prompt — T-311

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-311 && pwd && git branch --show-current   # must be slice/T-311
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target ; cargo xtask db up
═══ READ ═══
apps/website/api/src/handlers/telemetry/leaderboards.rs (all), the T-194 golden seed, docs/plans/t-311_plan.md
═══ PROBLEM ═══
Five ORDER BY arms (:47-51) have no secondary key; tied rows make LIMIT/OFFSET pages overlap and skip.
═══ SHIPPED ═══
Whitelisted ORDER BY (injection-safe) — keep the whitelist shape; only the strings change.
═══ LANGUAGE GATE ═══
Rust + SQL strings only.
═══ LOCKED ═══
- Tie-breaker `, lt.discord_id ASC` on every arm (or `lt.user_id ASC` if discord_id is nullable — report which).
- The paging test runs against the golden seed; `skip:` = FAIL.
═══ DO ═══
1. Paging test first; paste the red/flaky run on main. 2. Append the tie-breaker to all five arms.
3. Perturb one arm → red → restore → touch → green. 4. cargo xtask platform wave gate --slice T-311
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no new handler files; no query-shape changes beyond ORDER BY.
═══ VERIFY ═══
cargo test -p website-api leaderboards ; cargo xtask platform wave gate --slice T-311
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
