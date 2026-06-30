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
```

**Active slice:** T-090.1.2.2 (seams) — send Claude [`.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md`](t090_1_2_2_SEND_TO_CLAUDE.md)

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
