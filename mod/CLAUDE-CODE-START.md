# Claude Code — mod workbench entry

**Canonical context:** [`../../CLAUDE.md`](../../CLAUDE.md) at the monorepo root.

**Mod / Workbench queue:** [`../../docs/TICKET_MOD_QUEUE.md`](../../docs/TICKET_MOD_QUEUE.md)

**Workspace:** monorepo root (`TBD-Reforger/`). Mod scripts live under `mod/scripts/`; run deploy helpers from there:

```bash
cd mod
bash scripts/tbd-dev-bootstrap.sh
bash scripts/deploy-staging.sh --dry-run
```

**Rules unchanged:** production mod = `mod/tbd-framework/` only; never guess Enfusion APIs — use enfusion-mcp first.

Historical handoff detail was in this file pre-monorepo; live scheduling is in the ticket registry.
