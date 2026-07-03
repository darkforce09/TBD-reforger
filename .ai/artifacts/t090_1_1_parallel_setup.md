# T-090.1.1 setup — Map cartographic view

**Status:** **OPEN** — ready for Claude Code on `main`  
**Active slice:** **T-090.1.1** (registry `active_slice`)

---

## Stream

| Slice | CWD | Branch | Touches |
|-------|-----|--------|---------|
| **T-090.1.1** Map cartographic view | repo root | `main` | `scripts/map-assets/`, `packages/map-assets/everon/tiles/map/`, MC frontend basemap |

**Single stream** — no worktree required (unlike T-090.2 parallel taxonomy).

---

## Prompt

```bash
./scripts/ticket prompt T-090 --slice T-090.1.1
```

Send-off: [`.ai/artifacts/t090_1_1_SEND_TO_CLAUDE.md`](t090_1_1_SEND_TO_CLAUDE.md)

---

## Verify (post-ship)

```bash
make map-cartographic-everon
VIEW=map node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make schema-validate
make verify-terrain
cd apps/website/frontend && npm run build && npm run lint
```

Manual: Mission Settings → Map → alignment contact sheet (M3/M4) · FpsCounter (M9).

---

## Doc sync (Cursor, after Claude ships)

1. Registry slice → `shipped` + `shipped_at`
2. `active_slice` → **T-090.1.2.3** (prefetch) or operator picks **T-090.3**
3. CLAUDE.md Done bullet + hub/backlog
4. `./scripts/ticket sync && ./scripts/ticket check`
5. Doc-only commit (no tag on sync)

---

## Context chain

| Prior ship | Relevance |
|------------|-----------|
| T-090.1 @ `564419e` | MapDataExporter 4096² interim — **Map view source candidate** |
| T-090.1.2.4 @ `0d6fe485` | Stylized = NOT SAT — **IS Map** |
| T-090.1.2.5.2 @ `1c07d97a` | Water tint inputs · `decode-topo.mjs` |
| T-090.3.0 @ `b342c35` | K4 — real cartographic path; tile build = this slice |
| T-127 U3 @ `0515aabb` | Remove `map` coercion when tiles land |
