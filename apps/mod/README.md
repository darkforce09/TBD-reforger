# TBD Reforger Platform — Mod (`apps/mod/`)

Data-driven Arma Reforger event platform: missions are JSON from the website backend,
one greenfield Enfusion mod runs them all, and the web stack handles auth, events, and ORBAT.

**Repo:** [github.com/darkforce09/TBD-reforger](https://github.com/darkforce09/TBD-reforger) (monorepo — this folder is the Enfusion mod; see the root [`CLAUDE.md`](../../CLAUDE.md) for the platform-wide map and current status)

---

## Status (snapshot 2026-06-14 — current status lives in root `CLAUDE.md` §Status)

| Area | Status |
|---|---|
| Phase 0 spikes | REST 0.1 ✓ · Registry 0.4 ✓ · Schema **1.1** ✓ |
| `tbd-framework/` | Mission loader, slot spawn (`TBD_SpawnManager`), game mode prefab, dev scenario |
| Workbench slot spawn | ✓ Per-slot `slots[]` deploy verified (2026-06-14) |
| Dedicated server POC | Mission from API verified 2026-06-14 **against the Phase-0 REST spike backend, since removed** — the REST loader chain is **BLOCKED on T-092**; the `$profile:` file fallback is the working path |
| Web backend (Phase 1 API) | ✓ Missions, link codes, roster, ORBAT slot assignment (web `/api/v1`; game-server routes = **T-092**) |
| **Staging server** | `192.168.0.140` — **LAN Direct Join WORKS** (Workshop mod + `-config`; client joined + spawned 2026-06-14) — see [`documentation_v2/runbooks/game_server_staging/README.md`](/documentation_v2/runbooks/game_server_staging/README.md) |
| **Phase 1 in progress** | In-game admin **mission browser** (last 5%: input actions — CLAUDE-CONTINUATION.md §16), capture objective, ORBAT enforcement, admin UI |
| Milestone #1 target | **Sat 2026-08-22** — see [`MILESTONES.md`](/documentation_v2/archive/product_plans/mod_milestones.md) |

---

## Quick start

### Claude Code (Enfusion work)

1. Read [`documentation_v2/runbooks/mod_slice_workflow.md`](/documentation_v2/runbooks/mod_slice_workflow.md)
2. Run **`cargo xtask mod dev-bootstrap`** (from monorepo root) — launches Workbench on `tbd-export/addon.gproj` (`-gproj`, skips the project picker) and pre-warms the MCP daemon; the `EnfusionMCP` handlers are committed in [`tbd-emcp/`](tbd-emcp/) and nothing is copied any more
3. Enable **enfusion-mcp** before editing any `.c` file
4. Open `tbd-export/addon.gproj` in Workbench for export tooling sessions (pulls in `tbd-emcp`), or `tbd-framework/addon.gproj` for framework development. Never open the gitignored `Tbd_framework/` or `crf_framework/` reference copies

### Dedicated server (local POC)

```bash
# Prereq: Steam app 1874900 (Arma Reforger Server stable), API on :8080, Postgres for website
# From monorepo root:
cargo xtask setup server-profile      # default: apps/mod/.local-test-profile/
cargo xtask mod dev-server                    # -server + -addons (local mod)
```

Watch logs for `[TBD][Mission] loaded id=…`, 18× `[TBD][Slots] Slot-…`, then — once a client
joins — `[TBD] SpawnManager: assigned slot …`. (T-612: the old `[TBD] Mission loaded`,
`built slot spawn` and `spawn requested` lines are deleted; the only `Mission loaded` still
printed is the *failure* line `[TBD] Mission loaded but invalid — staying in LOADING.` Pin
tags + event keys, never sentences — see `cargo xtask mod remote-logs`.)

### Staging server (192.168.0.140)

```bash
cp tools_v2/xtask/deploy/deploy.env.example tools_v2/xtask/deploy/deploy.env   # fill SSH + token
cargo xtask deploy staging
```

See [`documentation_v2/runbooks/game_server_staging/README.md`](/documentation_v2/runbooks/game_server_staging/README.md). **Staging is Direct-Joinable** (Workshop mod + `-config`): set `TBD_SERVER_MODE=config` + `TBD_WORKSHOP_MOD_ID` in `deploy.env`, deploy, then Direct Join `192.168.0.140:2001` — the client auto-downloads the Workshop mod. (Legacy local-`-addons` join via `cargo xtask setup client-addons` is **not** Direct-Joinable — see STAGING-SERVER.md.) The V2–V4 API smoke gates are **skipped until T-092** (game-server REST routes not in the current backend).

### Website (local dev)

```bash
# From monorepo root (see root CLAUDE.md §Run it locally):
cargo xtask db up            # Postgres on :5434
cargo xtask mk rust-api      # Rust API on :8080
cargo xtask mk leptos        # Leptos Trunk SPA on :3000
cargo xtask mod test-phase1-api
```

---

## Repository map (monorepo)

| Path | Purpose |
|---|---|
| [`tbd-framework/`](tbd-framework/) | **Production Enfusion mod** (TBD-owned) — the shipping addon; carries no `Scripts/WorkbenchGame/` |
| [`tbd-export/`](tbd-export/) | Standalone addon — map-export, equipment and vehicle extraction tooling (`Scripts/WorkbenchGame/**`, road exporter, `TBD_Export_Everon.conf`). **Depends on** vanilla + `TBD_EMCP`; decoupled from framework |
| [`tbd-emcp/`](tbd-emcp/) | The committed enfusion-mcp Workbench Net API bridge handlers (`Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_*.c`, MIT, from `enfusion-mcp@0.6.1`) |
| [`contracts_v2/`](../../contracts_v2/) | Mission JSON schema, registry, golden missions, VOIP bridge contract |
| [`apps/website/`](../website/) | Rust API + Leptos SPA |
| `Tbd_framework/` | CRF reference only, **gitignored** — do not open in Workbench |
| [`tools_v2/xtask/`](../../tools_v2/xtask/) | Every `cargo xtask mod` command: Workbench setup, server profile, dev server, staging deploy, MCP bridge, API smokes |
| [`documentation_v2/mod/`](/documentation_v2/mod/) | Ops docs, [`STAGING-SERVER.md`](/documentation_v2/runbooks/game_server_staging/README.md) |

**Handoff docs:** [`CLAUDE-CONTINUATION.md`](/documentation_v2/archive/handoffs_and_kickoffs/mod_claude_continuation.md) · [`MILESTONES.md`](/documentation_v2/archive/product_plans/mod_milestones.md) · [`tbd-reforger-platform-build-plan.md`](/documentation_v2/archive/product_plans/tbd_reforger_platform_build_plan.md)

---

## Commands (run from the monorepo root)

| Command | Purpose |
|---------|---------|
| `cargo xtask mcp call` | JSON-RPC to enfusion-mcp from a shell |
| `cargo xtask mcp wb-logs` | Grep the latest Proton Workbench `console.log` |
| `cargo xtask mod spawn-verify` | `wb_play` plus a log grep for the spawn lines |
| `cargo xtask mod dev-bootstrap` | MCP root, `wb_connect` and `mod_validate` |
| `cargo xtask setup mcp-game-root` | Pak symlink farm the MCP reads |
| `cargo xtask deploy staging` | Rsync → 192.168.0.140, API, game server restart |
| `cargo xtask debug direct-join` | LAN join diagnostics (A2S, SSH, builds) |
| `cargo xtask setup client-addons` | Local client mod symlink (not Direct-Joinable; the Workshop mod is) |
| `cargo xtask mod remote-logs` | SSH log verify on the staging server |
| `cargo xtask mod bootstrap-staging` | One-time SSH discovery and mkdir |
| `cargo xtask setup server-profile` | Dedicated server profile and mission fallback |
| `cargo xtask mod dev-server` | Local dedicated server launcher |

---

## Key IDs

| Item | Value |
|---|---|
| Framework GUID (`TBD_Framework`) | `B2C3D4E5F6A78901` |
| Export GUID (`TBD_Export`) | `C3D4E5F6A7B89012` |
| EMCP GUID (`TBD_EMCP`) | `D4E5F6A7B8C90123` |
| Dev scenario | `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` |
| Dev world | `{F652B97A6F497348}worlds/TBD_Dev_POC.ent` (Eden subscene) |
| Golden mission | `msn_8f3a2c` (Bridgehead at Levie) |
| Dev server port | `2001` (when using `-server` mode) |

---

## What not to do

- Do not open or ship `Tbd_framework/` (60+ Coalition deps)
- Do not guess Enfusion APIs — use enfusion-mcp
- Do not call the MCP's `wb_launch` with `gprojPath` on `tbd-framework` or `tbd-export` — it injects a second copy of the handlers into that addon's `Scripts/WorkbenchGame/EnfusionMCP/` → Workbench "Multiple declaration" → bridge dead
- Do not call `wb_cleanup` with `apps/mod/tbd-emcp` — it `rm -rf`s the committed handlers
- Do not copy `tbd-framework` files into `tbd-export` — it is a dependency addon, not a mirror
- Do not use `-config` and `-addons` together for local dev mods
- Payments / Stripe are out of scope; VOIP is partner-owned (external app)
