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

## Workbench MCP setup (after clone)

`mod/tbd-framework/Scripts/WorkbenchGame/EnfusionMCP/` is **gitignored** (dev-only MCP bridge handlers). It is **not** part of the git merge — 0 tracked files in the original mod repo too.

Install locally after every fresh clone:

```bash
cd mod
bash scripts/tbd-dev-bootstrap.sh   # copies handlers from enfusion-mcp npm package
```

If Workbench is not running, bootstrap may exit after the copy step — that is fine. Expect ~19 `.c` files under `EnfusionMCP/`. Staging deploy excludes this tree (`deploy-staging.sh`).

Historical handoff detail was in this file pre-monorepo; live scheduling is in the ticket registry.
