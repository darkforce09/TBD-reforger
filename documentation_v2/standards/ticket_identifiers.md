**Status:** live

# T-0xx ticket naming contract

Every shipped feature, active slice, and queued/deferred item uses a **`T-0xx`** ticket ID. Former planning prefixes (frontend-deferred numbers, backend-deferred numbers, Eden parity tiers, engineering track letters, and A/B/C requirement codes) are **retired** — do not add them to authority docs.

## Source of truth

| Resource | Purpose |
|----------|---------|
| [`.ai/tickets/T-*.toml`](../../.ai/tickets/) | Canonical ticket rows (status, order, spec path, program); the tree root is marked by `.ai/tickets/ROOT` |
| [`docs/TICKET_REGISTRY.md`](../TICKET_REGISTRY.md) | Full generated table — all tickets |
| [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) | Lead dashboard — **ready**, **active**, next queued |
| [`docs/TICKET_DEV_QUEUE.md`](../TICKET_DEV_QUEUE.md) | Claude Code implementation queue |
| [`tickets/AI_PLAYBOOK.md`](../../.ai/tickets/AI_PLAYBOOK.md) | Edit ticket → `cargo xtask ticket sync` workflow |

After changing a ticket, run **`cargo xtask ticket sync`** from the repository root and commit the ticket file and the generated views together. Never hand-edit `docs/TICKET_*.md`.

## What T-0xx means

| Pattern | Meaning |
|---------|---------|
| **T-0xx** | Platform git milestone — one tag per ship (`T-067`, `T-068`, …) |
| **T-0xx.y** | Sub-slice within a ticket (e.g. **T-067.0** viewport cull, **T-067.1** lazy RAM @ 1M) |
| **T-0xx.y.z** | Hotfix sub-slice (e.g. **T-060.1.4** mid-upload socket reset) |

**Status values** (in each ticket file): `idea` → `queued` → `ready` → `shipped` | `deferred` | `cancelled`.

**Programs:** `platform`, `backend`, `eden`, `scale`, `infra` — see each ticket's `program` field.

## Domain ROADMAPs

Planning narrative lives in domain ROADMAPs; ticket IDs live in the ticket files.

| Domain | ROADMAP |
|--------|---------|
| Platform hub | [`documentation_v2/README.md`](/documentation_v2/README.md) |
| Frontend | [`documentation_v2/website/frontend/README.md`](/documentation_v2/website/frontend/README.md) |
| Backend | [`documentation_v2/website/api_v2/api_overview.md`](/documentation_v2/website/api_v2/api_overview.md) |
| Mission Creator | [`documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md`](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md) |

Supporting MC specs use **descriptive snake_case** filenames: `t067_spatial_chunks.md`, `engineering_plan.md`, `agent_execution.md`.

## Shipped MC specs (renamed paths)

Use these paths in links — old `eden_p1_*` / `track_a_*` slugs are obsolete.

| T-ID | Spec |
|------|------|
| T-048 | [`t048_library_create_dialog.md`](/documentation_v2/tickets/specs/t048_library_create_dialog.md) |
| T-049 | [`t049_terrain_title_position.md`](/documentation_v2/tickets/specs/t049_terrain_title_position.md) |
| T-050 | [`t050_cursor_z_readout.md`](/documentation_v2/tickets/specs/t050_cursor_z_readout.md) |
| T-052 | [`t052_undo_shortcuts.md`](/documentation_v2/tickets/specs/t052_undo_shortcuts.md) |
| T-053 | [`t053_additive_select.md`](/documentation_v2/tickets/specs/t053_additive_select.md) |
| T-054 | [`t054_attributes_entry_points.md`](/documentation_v2/tickets/specs/t054_attributes_entry_points.md) |
| T-055 | [`t055_asset_browser_search.md`](/documentation_v2/tickets/specs/t055_asset_browser_search.md) |
| T-056 | [`t056_copy_paste.md`](/documentation_v2/tickets/specs/t056_copy_paste.md) |
| T-057 … T-067 | [`t057_map_performance_hotfix.md`](/documentation_v2/tickets/specs/t057_map_performance_hotfix.md) … [`t067_spatial_chunks.md`](/documentation_v2/tickets/specs/t067_spatial_chunks.md) |
| **T-068** | [`t068_asset_registry.md`](/documentation_v2/tickets/specs/t068_asset_registry.md) (**ready**) |

Full shipped scale-program table: [`docs/TICKET_REGISTRY.md`](../TICKET_REGISTRY.md).

## Deferred / absorbed tickets

- **Title PATCH sync** — scope lives under **T-089** (absorbs former T-051; no separate T-051 row).
- **Typed-array IconLayer** — **T-094** (was T-061.1 in prose).
- **Terrain base + sparse deltas** — **T-110** ([`t110_terrain_base_mission_layers.md`](/documentation_v2/tickets/specs/t110_terrain_base_mission_layers.md)).
- Platform/backend deferred items (**T-085** wiki markdown, **T-086** server control, **T-095** API reference, **T-096** telemetry bridge, …) — see [`TICKET_REGISTRY.md`](../TICKET_REGISTRY.md) `deferred` rows.

## T-0xx vs engineering phases

Ticket IDs are git milestones — one tag per ship; [`TICKET_REGISTRY.md`](../TICKET_REGISTRY.md) lists them all.
[`engineering_plan.md`](/documentation_v2/archive/go_and_react_era_design/mission_creator_engineering_plan.md) phases 0–9 are engineering design — not 1:1 with ticket order.

## Adding or changing tickets

1. Edit the ticket's [`.ai/tickets/T-*.toml`](../../.ai/tickets/) file.
2. `cargo xtask ticket sync` (regenerates `docs/TICKET_*.md` and the queue).
3. `cargo xtask ticket check --strict`.
4. Sync narrative docs per [`AGENT_COMMIT_CHECKLIST.md`](/documentation_v2/standards/commit_checklist.md).

Do **not** invent a new prefix. If work is not shipped, keep it **`queued`**, **`ready`**, or **`deferred`** in its ticket file — never reuse a shipped T-ID for new scope.
