# T-090.1.2.x — Operator resume (when you come back)

**One-page cheat sheet.** Full detail: [`docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md)

## What's broken today

- **Seams** — 256 m grid lines (SAP cell paste)
- **Pan** — ~40 fps + flicker
- **Water** — none (grey ocean, dry inland)
- **Detail** — OK after T-090.1.2.1 (don't chase resolution now)

## What to run first

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
```

**Active slice:** **T-090.1.2.2** (seams)

```bash
./scripts/ticket prompt T-090          # canonical — paste into Claude Code
./scripts/ticket prompt T-090 --header # includes spec/handoff paths
./scripts/ticket brief T-090
```

Send-off bookmark: [`.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md`](t090_1_2_2_SEND_TO_CLAUDE.md) · standard: [`.ai/tickets/CLAUDE_CODE_PROMPT.md`](../tickets/CLAUDE_CODE_PROMPT.md)

### T-090.1.2.2 Claude checklist (seams)

1. P0 `analyze-sap-seams.mjs` → baseline JSON
2. Fix `stitch-sap-ortho.mjs` (edge feather default)
3. `verify-sap-seams.mjs` + `verify-sap-ortho.mjs` PASS
4. Rebuild lossless pyramid (~299M LFS)
5. Tag **`T-090.1.2.2`** → tell Cursor **"doc sync for T-090.1.2.2"**

**Staging ortho** (gitignored): `packages/map-assets/everon/staging/sap/everon-sap-ortho.png` — re-stitch from pak if missing.

## Then (order)

| # | Slice | Parallel? | Send-off file |
|---|-------|-----------|---------------|
| 1 | T-090.1.2.2 seams | — | `t090_1_2_2_SEND_TO_CLAUDE.md` |
| 2 | T-090.1.2.3 prefetch | **Yes** (FE only) | `t090_1_2_3_SEND_TO_CLAUDE.md` |
| 3 | T-090.1.2.5 water | After #1 ortho | `t090_1_2_5_SEND_TO_CLAUDE.md` |
| 4 | T-090.1.1 Map view | After satellite backlog | `./scripts/ticket brief T-090` |

**Parked:** T-090.1.2.4 engine render (idea) — ignore unless you promote it.

## After Claude ships any slice

Tell Cursor: **"doc sync for T-090.1.2.x"** — don't hand-edit TICKET_*.md.
