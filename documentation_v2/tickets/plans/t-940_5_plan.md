# T-940.5 — Plan

## Context
db.rs:31 max_connections(25); :32-34 fixed idle/lifetime/acquire timeouts; nothing in config.rs or .env.example.

## Approach
1. Verify on main: TBD_DB_POOL_MAX_CONNECTIONS=3 still yields 25; paste the red.
2. config.rs: `DbPoolConfig` from TBD_DB_POOL_* with today's literals as defaults; parse tests.
3. db.rs builds the pool from it; invalid values fail startup naming the variable.
4. .env.example documents the four variables.
5. Perturbation: ignore the override → parse test red; restore, touch, green.

## Risks
- Test harness pools: keep defaults identical so test-it timing is unchanged.

## Verification
- `cargo xtask db test-it`
- `cargo xtask platform wave gate --slice T-940.5`
