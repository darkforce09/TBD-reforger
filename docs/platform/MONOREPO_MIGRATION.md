# Monorepo Migration Runbook

Migration program **M0–M3** merges `TBD_Website` + `Arma reforger` into this repo without rewriting individual commit SHAs from the source repos (read-tree merge).

## Gates

| Gate | When | Checks |
|------|------|--------|
| **G0.5** | Before M1 | Both originals committed; SHAs in [`artifacts/migration-baseline/G0-SHAs.md`](../artifacts/migration-baseline/G0-SHAs.md); push to GitHub |
| **G1** | After read-tree merge | V1–V3 content parity |
| **G2** | After M1b lift | V4–V5, V17, V25 |
| **G3** | After M1c path fixes | V11–V22; executor gate in CLAUDE + AI_PLAYBOOK |
| **G4** | After M2 ticket rewrite | V6–V10, V23–V27; `./scripts/ticket check --strict` |
| **G5** | Before workspace switch | Full `./scripts/verify-monorepo-migration.sh` exit 0 |

Run verification:

```bash
make verify-migration
# or
./scripts/verify-monorepo-migration.sh
```

## Layout after migration

```
TBD-Reforger/
├── website/          # Go API + React frontend (was TBD_Website)
├── mod/              # tbd-framework + deploy scripts (was Arma reforger)
├── shared/tbd-schema/
├── docs/specs/       # lifted Design_Docs
├── tickets/          # unified registry
├── scripts/ticket
└── CLAUDE.md         # canonical agent context
```

## Path rewrites

| Old | New |
|-----|-----|
| Legacy path (pre-monorepo) | Monorepo path |
|----------------------------|---------------|
| Design_Docs tree | `docs/specs/` |
| `mod/tbd-schema/` | `shared/tbd-schema/` |
| embedded web tree | `website/` |
| `../worktrees` | `artifacts/worktrees` |

Scripts: `scripts/rewrite-ticket-paths.py`, `scripts/rewrite-doc-links.py`, `scripts/backfill-registry-monorepo.py`.

## Post-G5

1. Create GitHub repo `darkforce09/TBD-Reforger`
2. Push monorepo `main`
3. Archive old remotes (read-only)
4. Point Cursor workspace at `/home/Samuel/Projects/TBD-Reforger/`
5. Resume normal split: Cursor docs, Claude Code `executor: claude-code` slices

## Manual steps

- Copy `crf_framework/` into `mod/crf_framework/` (gitignored, local Workbench reference)
- Update staging server clone path after cutover
