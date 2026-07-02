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
| **Staging server** | `192.168.0.140` — **LAN Direct Join WORKS** (Workshop mod + `-config`; client joined + spawned 2026-06-14) — see [`docs/mod/STAGING-SERVER.md`](../../docs/mod/STAGING-SERVER.md) |
| **Phase 1 in progress** | In-game admin **mission browser** (last 5%: input actions — CLAUDE-CONTINUATION.md §16), capture objective, ORBAT enforcement, admin UI |
| Milestone #1 target | **Sat 2026-08-22** — see [`MILESTONES.md`](../../docs/mod/MILESTONES.md) |

---

## Quick start

### Claude Code (Enfusion work)

1. Read [`docs/mod/CLAUDE-CODE-START.md`](../../docs/mod/CLAUDE-CODE-START.md)
2. Run **`bash scripts/mod/tbd-dev-bootstrap.sh`** (from monorepo root) — installs gitignored `EnfusionMCP` handlers from the `enfusion-mcp` npm package (not in git)
3. Enable **enfusion-mcp** before editing any `.c` file
4. Open **only** `tbd-framework/addon.gproj` in Workbench — never the gitignored `Tbd_framework/` or `crf_framework/` reference copies

### Dedicated server (local POC)

```bash
# Prereq: Steam app 1874900 (Arma Reforger Server stable), API on :8080, Postgres for website
# From monorepo root:
bash scripts/mod/setup-server-profile.sh      # default: .local-test-profile/
bash scripts/mod/run-dev-server.sh            # -server + -addons (local mod)
```

Watch logs for `[TBD] Mission loaded`, 18× `built slot spawn`, `assigned slot`, `spawn requested`.

### Staging server (192.168.0.140)

```bash
cp scripts/deploy/deploy.env.example scripts/deploy/deploy.env   # fill SSH + token
bash scripts/mod/deploy-staging.sh
```

See [`docs/mod/STAGING-SERVER.md`](../../docs/mod/STAGING-SERVER.md). **Staging is Direct-Joinable** (Workshop mod + `-config`): set `TBD_SERVER_MODE=config` + `TBD_WORKSHOP_MOD_ID` in `deploy.env`, deploy, then Direct Join `192.168.0.140:2001` — the client auto-downloads the Workshop mod. (Legacy local-`-addons` join via `setup-client-addons.sh` is **not** Direct-Joinable — see STAGING-SERVER.md.) The V2–V4 API smoke gates are **skipped until T-092** (game-server REST routes not in the current backend).

### Website (local dev)

```bash
# From monorepo root (see root CLAUDE.md §Run it locally):
make db-up    # Postgres on :5434
make api      # Go API on :8080
make web      # Vite dev server on :5173
bash scripts/mod/test-phase1-api.sh
```

---

## Repository map (monorepo)

| Path | Purpose |
|---|---|
| [`tbd-framework/`](tbd-framework/) | **Production Enfusion mod** (TBD-owned) |
| [`packages/tbd-schema/`](../../packages/tbd-schema/) | Mission JSON schema, registry, golden missions, VOIP bridge contract |
| [`apps/website/`](../website/) | Go API + React UI |
| `Tbd_framework/` | CRF reference only, **gitignored** — do not open in Workbench |
| [`scripts/mod/`](../../scripts/mod/) | Workbench setup, server profile, dev server, staging deploy, MCP helpers, API tests |
| [`docs/mod/`](../../docs/mod/) | Ops docs, [`STAGING-SERVER.md`](../../docs/mod/STAGING-SERVER.md) |

**Handoff docs:** [`CLAUDE-CONTINUATION.md`](../../docs/mod/CLAUDE-CONTINUATION.md) · [`MILESTONES.md`](../../docs/mod/MILESTONES.md) · [`tbd-reforger-platform-build-plan.md`](../../docs/mod/tbd-reforger-platform-build-plan.md)

---

## Scripts (`scripts/mod/`, run from monorepo root)

| Script | Purpose |
|--------|---------|
| [`mcp-call.sh`](../../scripts/mod/mcp-call.sh) | JSON-RPC to enfusion-mcp from shell |
| [`mcp-wb-logs.sh`](../../scripts/mod/mcp-wb-logs.sh) | Grep latest Proton Workbench `console.log` |
| [`tbd-spawn-verify.sh`](../../scripts/mod/tbd-spawn-verify.sh) | MCP `wb_play` + log grep for spawn lines |
| [`tbd-dev-bootstrap.sh`](../../scripts/mod/tbd-dev-bootstrap.sh) | MCP root + `wb_connect` + `mod_validate` |
| [`setup-mcp-game-root.sh`](../../scripts/mod/setup-mcp-game-root.sh) | Pak symlink farm for MCP |
| [`deploy-staging.sh`](../../scripts/mod/deploy-staging.sh) | Rsync → 192.168.0.140, API, game server restart |
| [`debug-direct-join.sh`](../../scripts/mod/debug-direct-join.sh) | LAN join diagnostics (A2S, SSH, builds) |
| [`setup-client-addons.sh`](../../scripts/mod/setup-client-addons.sh) | **Legacy** — local client mod symlink (not Direct-Joinable; use the Workshop mod instead) |
| [`remote-log-grep.sh`](../../scripts/mod/remote-log-grep.sh) | SSH log verify on staging server |
| [`bootstrap-staging-server.sh`](../../scripts/mod/bootstrap-staging-server.sh) | One-time SSH discovery + mkdir |
| [`setup-server-profile.sh`](../../scripts/mod/setup-server-profile.sh) | Dedicated server profile + mission fallback |
| [`run-dev-server.sh`](../../scripts/mod/run-dev-server.sh) | Local dedicated server launcher |

---

## Key IDs

| Item | Value |
|---|---|
| Mod GUID | `B2C3D4E5F6A78901` |
| Dev scenario | `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` |
| Dev world | `{F652B97A6F497348}worlds/TBD_Dev_POC.ent` (Eden subscene) |
| Golden mission | `msn_8f3a2c` (Bridgehead at Levie) |
| Dev server port | `2001` (when using `-server` mode) |

---

## What not to do

- Do not open or ship `Tbd_framework/` (60+ Coalition deps)
- Do not guess Enfusion APIs — use enfusion-mcp
- Do not use `-config` and `-addons` together for local dev mods
- Payments / Stripe are out of scope; VOIP is partner-owned (external app)
