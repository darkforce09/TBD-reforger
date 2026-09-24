# T-138 — Plan

## Context
The self-host story is a manual runbook (docs/website/DEV_RUNBOOK.md:7); forks and new contributors need one
command. The repo forbids committed shell/Python, so the installer is an xtask subcommand.

## Approach
1. New `xtask/src/setup_all.rs`: `Step { name, check: fn -> bool, run: fn -> Result }` for deps (docker, psql,
   git-lfs, trunk), `.env` from `.env.example`, `db up`, `migrate`, `seed`, optional `--map-assets` (git lfs pull);
   `--dry-run` prints the plan. Planner unit tests with a fake environment.
2. `xtask/src/main.rs`: register `setup all` (call site only).
3. `docs/website/DEV_RUNBOOK.md`: "Start everything" leads with the command; manual list stays as fallback.
4. Perturbation: deps check ignores a missing tool → planner test red; restore, `touch`, green.

## Risks
- Existing db/migrate subcommands have their own arg parsing; call their functions, do not shell out to cargo.
- Fallback: `setup all` stops at the first failed step with the manual runbook line for that step.

## Verification
- `cargo test -p xtask setup_all` · `cargo xtask setup all --dry-run` · `cargo xtask platform wave gate --slice T-138`
