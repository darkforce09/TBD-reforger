# TBD Reforger Platform

Data-driven Arma Reforger event platform: missions are JSON from the website backend,
one greenfield Enfusion mod runs them all, and the web stack handles auth, events, and ORBAT.

**Repo:** [github.com/darkforce09/tbd-reforger-platform](https://github.com/darkforce09/tbd-reforger-platform)

---

## Status (2026-06-14)

| Area | Status |
|---|---|
| Phase 0 spikes | REST 0.1 ✓ · Registry 0.4 ✓ · Schema **1.1** ✓ |
| `tbd-framework/` | Mission loader, slot spawn (`TBD_SpawnManager`), game mode prefab, dev scenario |
| Workbench slot spawn | ✓ Per-slot `slots[]` deploy verified (2026-06-14) |
| Dedicated server POC | ✓ Mission from API on Linux host |
| Web backend (Phase 1 API) | ✓ Missions, link codes, roster, ORBAT slot assignment |
| **Staging server** | Deployed on `192.168.0.140` — **LAN join in progress** — see [`docs/STAGING-SERVER.md`](docs/STAGING-SERVER.md) |
| **Phase 1 in progress** | ORBAT enforcement, capture objective, admin UI |
| Milestone #1 target | **Sat 2026-08-22** — see [`MILESTONES.md`](MILESTONES.md) |

---

## Quick start

### Claude Code (Enfusion work)

1. Read [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md)
2. Enable **enfusion-mcp** before editing any `.c` file
3. Open **only** `tbd-framework/addon.gproj` in Workbench — never `Tbd_framework/`

### Dedicated server (local POC)

```bash
# Prereq: Steam app 1874900 (Arma Reforger Server stable), API on :8080, Postgres for website
bash scripts/setup-server-profile.sh          # default: .local-test-profile/
bash scripts/run-dev-server.sh                # -server + -addons (local mod)
```

Watch logs for `[TBD] Mission loaded`, 18× `built slot spawn`, `assigned slot`, `spawn requested`.

### Staging server (192.168.0.140)

```bash
cp scripts/deploy.env.example scripts/deploy.env   # fill SSH + token
bash scripts/deploy-staging.sh
```

See [`docs/STAGING-SERVER.md`](docs/STAGING-SERVER.md). Client: `bash scripts/setup-client-addons.sh` → Direct Join `192.168.0.140:2001`.

### Website (local dev)

```bash
podman start tbdevent-postgres                # or docker compose up -d
cd Tbdevent_Website && make dev-api           # :8080
cd Tbdevent_Website/web && npm run dev        # :5173
bash scripts/test-phase1-api.sh
```

---

## Repository map

| Path | Purpose |
|---|---|
| [`tbd-framework/`](tbd-framework/) | **Production Enfusion mod** (TBD-owned) |
| [`tbd-schema/`](tbd-schema/) | Mission JSON schema, registry, golden missions, VOIP bridge contract |
| [`Tbdevent_Website/`](Tbdevent_Website/) | Go API + React UI |
| [`Tbd_framework/`](Tbd_framework/) | CRF reference only — **do not open in Workbench** |
| [`scripts/`](scripts/) | Workbench setup, server profile, dev server, staging deploy, MCP helpers, API tests |
| [`docs/`](docs/) | Ops copy, [`STAGING-SERVER.md`](docs/STAGING-SERVER.md) |

**Handoff docs:** [`CLAUDE-CONTINUATION.md`](CLAUDE-CONTINUATION.md) · [`MILESTONES.md`](MILESTONES.md) · [`tbd-reforger-platform-build-plan.md`](tbd-reforger-platform-build-plan.md)

---

## Scripts

| Script | Purpose |
|--------|---------|
| [`scripts/mcp-call.sh`](scripts/mcp-call.sh) | JSON-RPC to enfusion-mcp from shell |
| [`scripts/mcp-wb-logs.sh`](scripts/mcp-wb-logs.sh) | Grep latest Proton Workbench `console.log` |
| [`scripts/tbd-spawn-verify.sh`](scripts/tbd-spawn-verify.sh) | MCP `wb_play` + log grep for spawn lines |
| [`scripts/tbd-dev-bootstrap.sh`](scripts/tbd-dev-bootstrap.sh) | MCP root + `wb_connect` + `mod_validate` |
| [`scripts/setup-mcp-game-root.sh`](scripts/setup-mcp-game-root.sh) | Pak symlink farm for MCP |
| [`scripts/deploy-staging.sh`](scripts/deploy-staging.sh) | Rsync → 192.168.0.140, API, game server restart |
| [`scripts/debug-direct-join.sh`](scripts/debug-direct-join.sh) | LAN join diagnostics (A2S, SSH, builds) |
| [`scripts/setup-client-addons.sh`](scripts/setup-client-addons.sh) | Client mod symlink + Steam launch options |
| [`scripts/remote-log-grep.sh`](scripts/remote-log-grep.sh) | SSH log verify on staging server |
| [`scripts/bootstrap-staging-server.sh`](scripts/bootstrap-staging-server.sh) | One-time SSH discovery + mkdir |
| [`scripts/setup-server-profile.sh`](scripts/setup-server-profile.sh) | Dedicated server profile + mission fallback |
| [`scripts/run-dev-server.sh`](scripts/run-dev-server.sh) | Local dedicated server launcher |

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
