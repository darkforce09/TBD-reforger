# T-0xx ticket naming contract

Every shipped feature, active slice, and queued/deferred item uses a **`T-0xx`** ticket ID. Former planning prefixes (frontend-deferred numbers, backend-deferred numbers, Eden parity tiers, engineering track letters, and A/B/C requirement codes) are **retired** — do not add them to authority docs.

## Source of truth

| Resource | Purpose |
|----------|---------|
| [`tickets/registry.json`](../../.ai/tickets/registry.json) | Canonical ticket rows (status, order, spec path, program) |
| [`docs/TICKET_REGISTRY.md`](../TICKET_REGISTRY.md) | Full generated table — all tickets |
| [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) | Lead dashboard — **ready**, **active**, next queued |
| [`docs/TICKET_DEV_QUEUE.md`](../TICKET_DEV_QUEUE.md) | Claude Code implementation queue |
| [`tickets/AI_PLAYBOOK.md`](../../.ai/tickets/AI_PLAYBOOK.md) | Edit registry → `./scripts/ticket sync` workflow |

After changing `registry.json`, run **`./scripts/ticket sync`** from monorepo root and commit registry + generated views together. Never hand-edit root `docs/TICKET_*.md` or the `<!-- ticket-sync:status -->` block in root `CLAUDE.md`.

## What T-0xx means

| Pattern | Meaning |
|---------|---------|
| **T-0xx** | Platform git milestone — one tag per ship (`T-067`, `T-068`, …) |
| **T-0xx.y** | Sub-slice within a ticket (e.g. **T-067.0** viewport cull, **T-067.1** lazy RAM @ 1M) |
| **T-0xx.y.z** | Hotfix sub-slice (e.g. **T-060.1.4** mid-upload socket reset) |

**Status values** (in registry): `idea` → `queued` → `ready` → `shipped` | `deferred` | `cancelled`.

**Programs:** `platform`, `backend`, `eden`, `scale`, `infra` — see registry `program` column.

## Domain ROADMAPs

Planning narrative lives in domain ROADMAPs; ticket IDs live in the registry.

| Domain | ROADMAP |
|--------|---------|
| Platform hub | [`docs/README.md`](README.md) |
| Frontend | [`docs/website/frontend/ROADMAP.md`](frontend/ROADMAP.md) |
| Backend | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| Mission Creator | [`docs/specs/Mission_Creator_Architecture/ROADMAP.md`](../specs/Mission_Creator_Architecture/ROADMAP.md) |

Supporting MC specs use **descriptive snake_case** filenames: `t067_spatial_chunks.md`, `engineering_plan.md`, `agent_execution.md`.

## Shipped MC specs (renamed paths)

Use these paths in links — old `eden_p1_*` / `track_a_*` slugs are obsolete.

| T-ID | Spec |
|------|------|
| T-048 | [`t048_library_create_dialog.md`](../specs/Mission_Creator_Architecture/t048_library_create_dialog.md) |
| T-049 | [`t049_terrain_title_position.md`](../specs/Mission_Creator_Architecture/t049_terrain_title_position.md) |
| T-050 | [`t050_cursor_z_readout.md`](../specs/Mission_Creator_Architecture/t050_cursor_z_readout.md) |
| T-052 | [`t052_undo_shortcuts.md`](../specs/Mission_Creator_Architecture/t052_undo_shortcuts.md) |
| T-053 | [`t053_additive_select.md`](../specs/Mission_Creator_Architecture/t053_additive_select.md) |
| T-054 | [`t054_attributes_entry_points.md`](../specs/Mission_Creator_Architecture/t054_attributes_entry_points.md) |
| T-055 | [`t055_asset_browser_search.md`](../specs/Mission_Creator_Architecture/t055_asset_browser_search.md) |
| T-056 | [`t056_copy_paste.md`](../specs/Mission_Creator_Architecture/t056_copy_paste.md) |
| T-057 … T-067 | [`t057_map_performance_hotfix.md`](../specs/Mission_Creator_Architecture/t057_map_performance_hotfix.md) … [`t067_spatial_chunks.md`](../specs/Mission_Creator_Architecture/t067_spatial_chunks.md) |
| **T-068** | [`t068_asset_registry.md`](../specs/Mission_Creator_Architecture/t068_asset_registry.md) (**ready**) |

Full shipped scale-program table: [`docs/TICKET_REGISTRY.md`](../TICKET_REGISTRY.md).

## Deferred / absorbed tickets

- **Title PATCH sync** — scope lives under **T-089** (absorbs former T-051; no separate T-051 row).
- **Typed-array IconLayer** — **T-094** (was T-061.1 in prose).
- **Terrain base + sparse deltas** — **T-110** ([`t110_terrain_base_mission_layers.md`](../specs/Mission_Creator_Architecture/t110_terrain_base_mission_layers.md)).
- Platform/backend deferred items (**T-085** wiki markdown, **T-086** server control, **T-095** API reference, **T-096** telemetry bridge, …) — see [`TICKET_REGISTRY.md`](../TICKET_REGISTRY.md) `deferred` rows.

## T-0xx vs engineering phases

[`CLAUDE.md`](../../CLAUDE.md) **T-029–T-040** = git milestones for the MC shell.  
[`engineering_plan.md`](../specs/Mission_Creator_Architecture/engineering_plan.md) phases 0–9 = engineering design — not 1:1 with ticket order.

## Adding or changing tickets

1. Edit [`tickets/registry.json`](../../.ai/tickets/registry.json).
2. `./scripts/ticket sync` (regenerates `TICKET_*.md`, `CLAUDE.md` status block, queue).
3. `./scripts/ticket check` (or `make ticket-check-strict` before doc-only merges).
4. Sync narrative docs per [`AGENT_COMMIT_CHECKLIST.md`](AGENT_COMMIT_CHECKLIST.md).

Do **not** invent a new prefix. If work is not shipped, keep it **`queued`**, **`ready`**, or **`deferred`** in the registry — never reuse a shipped T-ID for new scope.
