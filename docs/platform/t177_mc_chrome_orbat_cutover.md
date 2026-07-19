# T-177 — MC chrome UX + ORBAT dock cutover (T-071.0)

**Status:** SHIPPED @ tag **T-177** / `e97a01c6` · **Branch:** `main`  
**Depends on:** T-176 (shipped) · **Implements:** T-071.0 (modal shell + left → Editor Layers only)  
**Verify:** [`.ai/artifacts/t177_verify_log.md`](../../.ai/artifacts/t177_verify_log.md) · inventory [`.ai/artifacts/t177_inventory.md`](../../.ai/artifacts/t177_inventory.md)  
**Program hub (ORBAT):** [`t071_orbat_manager_program.md`](../specs/Mission_Creator_Architecture/t071_orbat_manager_program.md)  
**Evidence:** [`.ai/artifacts/t177_operator_screens/`](../../.ai/artifacts/t177_operator_screens/)  
**Scope shipped:** `apps/website/frontend/**`, gate harness (`cdp.rs`, `gate doctor`), CI/toolchain pins. **Not** `apps/mod/`. **Not** T-071.1–.4.

## Shipped outcome

| ID | Result |
|----|--------|
| A1 | YouTube tree connectors — `guide_spans(&[bool])` ancestor spines trim at last child + rounded elbow; `FlatRow.ancestors` in `flatten_visible`. |
| A2 | Grab cursor on placeable assets (`PALETTE_LEAF`); folders/outliner untouched. |
| A3 | Top menus above docks — strip `z-30` / docks `z-20`. |
| B1 | Left ORBAT tree **removed** — Editor Layers only. |
| B2 | Top-strip **ORBAT Manager** → `OrbatManagerDialog` (live faction→squad→slot; select / dbl-click→Attributes). |

**T-071.0** shipped via this ticket. **T-071.1+** remain (squad CRUD, slot numbering/export, logos/standardizations/Arsenal).

### Gate harness (same commit)

`make leptos-gates` was wedging ~130 s on `chrome-headless-shell` (SkFontMgr FATAL). Fixed: full Chrome + `--headless=new`. **`gate doctor`** fail-fast preflight (pins, RAM/orphans, ~15 s liveness). Also: `gate-env.json`, root `rust-toolchain.toml`, CI `@stable`→**1.95.0**, `editor-gates.yml`, [`EDITOR_GATE_RUNBOOK.md`](../website/EDITOR_GATE_RUNBOOK.md), KB-002. Caveat: `editor-gates.yml` needs a first `workflow_dispatch` on GitHub.

**Gates:** `make leptos-gates` 20/20 · `make ci-local` · 74 frontend tests · fmt/clippy clean.

## Why (pre-ship)

YouTube-style lines; grab cursor; menus behind Outliner; remove left ORBAT split; ORBAT Manager button → modal (T-071.0).

## Operator matrix (met)

| ID | Ask | Shipped |
|----|-----|---------|
| A1 | YouTube connectors | Elbow + spine guides |
| A2 | Grab on assets | `PALETTE_LEAF` cursor |
| A3 | Menus above docks | z-30 strip / z-20 docks |
| B1–B2 | ORBAT cutover | Dock removal + `OrbatManagerDialog` |

## Remaining (T-071 program)

| Slice | Focus |
|-------|--------|
| **T-071.1** | Squad CRUD; move slot between squads |
| **T-071.2** | Slot numbering + slotting order in export |
| **T-071.3–.4** | Logos, standardizations, per-slot Arsenal link |
