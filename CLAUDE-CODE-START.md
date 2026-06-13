# Claude Code — start here (Workbench first)

> **Read this file first** in Claude Code. Supersedes outdated API assumptions from 2023-era training data.
>
> **Workspace:** `/home/Samuel/Projects/Arma reforger`  
> **Last updated:** 2026-06-13

---

## Critical rule

**Never guess Enfusion APIs.** Your training data is ~3 years behind the live game.

Before writing or fixing any `.c` script:

1. Connect **enfusion-mcp** (below).
2. Run `api_search` / `wiki_read` for the class or method (e.g. `RestCallback`, `JsonLoadContext`).
3. Match patterns to **current** wiki + index, not CRF samples alone.

CRF (`Tbd_framework/`) is **read-only reference in the editor** — **never open in Workbench**, never add as a dependency.

---

## What to open in Workbench

| Open | Do not open |
|---|---|
| `tbd-framework/addon.gproj` | `Tbd_framework/addon.gproj` (60+ Coalition deps) |
| Base game `ArmaReforger.gproj` | — |

Production mod = **`tbd-framework/`** only. Vanilla dependency: `58D0FB3206B6F859`.

---

## Step 0 — Claude Code + MCP

```bash
cd "/home/Samuel/Projects/Arma reforger"

# One-time: add Enfusion MCP to Claude Code
claude mcp add --scope user enfusion-mcp -- npx -y enfusion-mcp

# Start session
claude "Read CLAUDE-CODE-START.md. Help me get Workbench running, then verify RestCallback API before touching TBD_MissionLoader.c"
```

Cursor users: same server in `.cursor/mcp.json` (already present).

---

## Step 1 — Sync game and tools (fixes Tuple2 / “Can't initialize game”)

Your Steam builds **must match**. Mismatched game vs Workbench causes vanilla script errors like `Can't find class 'Tuple2'` — not a TBD bug.

```bash
# Check build IDs (should be equal or tools TargetBuildID = game buildid)
grep buildid ~/.local/share/Steam/steamapps/appmanifest_1874880.acf   # Arma Reforger
grep buildid ~/.local/share/Steam/steamapps/appmanifest_1874910.acf   # Tools
```

**Action:** In Steam, update **both** Arma Reforger and **Arma Reforger Tools** until Workbench log shows the same major version (not 1.7.0.49 / 2023-05-03).

Then: launch **Arma Reforger (game)** once → quit → open **Workbench**.

---

## Step 2 — Base game path (Linux / Proton)

Proton Workbench often cannot browse `~/.local/share/Steam/...`.

```bash
bash scripts/setup-workbench-linux.sh
```

In **Locate base game**, select:

- **Proton:** `Z:\home\Samuel\ArmaReforger-Base\data\ArmaReforger.gproj`
- **Linux:** `/home/Samuel/ArmaReforger-Base/data/ArmaReforger.gproj`

In launcher: **+ Add Project → Add Existing** → same `ArmaReforger.gproj`, then **`tbd-framework/addon.gproj`**.

---

## Step 3 — Server profile (mission loader REST test)

```bash
bash scripts/setup-server-profile.sh ~/.local/share/ArmaReforger/profile
# or: bash scripts/setup-server-profile.sh "/home/Samuel/Projects/Arma reforger/.local-test-profile"
```

Creates `TBD_BackendConfig.json` + `missions/msn_8f3a2c.json` in that profile.

Ensure website API is running and `.env` has `GAME_SERVER_TOKENS=dev-server-token-change-in-prod`.

---

## Step 4 — First Workbench success criteria

1. Open **TBD_Framework** — scripts compile (no red errors in **TBD/** paths).
2. If `TBD_MissionLoader.c` fails: **look up `RestCallback` in enfusion-mcp** and fix — do not use 2023 docs.
3. Attach to GameMode entity:
   - `TBD_FrameworkManager`
   - `TBD_RegistryPocComponent`
4. Load `Missions/TBD_Dev_POC.conf` (or your scenario).
5. Dedicated host → log shows registry POC spawns + mission load (REST or profile fallback).

---

## Step 5 — Web stack (already running from Cursor work)

| Service | URL |
|---|---|
| API | http://127.0.0.1:8080 |
| UI dev | http://127.0.0.1:5173 |
| Postgres | `podman start tbdevent-postgres` (port **5433** in `.env`) |

```bash
bash scripts/test-phase1-api.sh          # game-server API smoke test
bash scripts/manual-test.sh              # full suite (needs DB)
```

Migration **00004** applied: missions, link codes, ORBAT slots.

---

## Known script fix (verify API before keeping)

`TBD_MissionLoader.c` had `RestCallback.SetOnTimeout` — **removed** (not on current API per CRF usage). Re-verify with enfusion-mcp on your installed version; timeouts may only surface via `SetOnError`.

---

## Repo map (after Workbench is green)

| Path | Purpose |
|---|---|
| `tbd-framework/` | **Your Enfusion mod** — all new script work here |
| `tbd-schema/` | Mission JSON, registry, bridge contract |
| `Tbdevent_Website/` | Go API + React UI |
| `Tbd_framework/` | CRF reference — **do not ship, do not open in WB** |
| `tbd-reforger-platform-build-plan.md` | Full vision (do not edit plan file per user rule) |
| `CLAUDE-CONTINUATION.md` | Longer context, phases, API table |
| `MILESTONES.md` | Milestone #1: Sat 2026-08-22 |

---

## Phase order (after Workbench POC)

1. Registry POC spawns 5 aliases in-world  
2. Mission loader hits `GET /api/missions/{id}/compiled` on dedicated server  
3. Framework: state machine, spawner, capture, ORBAT enforcement  
4. Web admin UI for mission upload + slot assignment  
5. Milestone #1 event (20–40 players)

VOIP = partner track. Mission Wizard = Phase 2.

---

## First Claude Code prompt (copy/paste)

```
Read CLAUDE-CODE-START.md and CLAUDE-CONTINUATION.md section 2 (overrides).

Goal: Workbench opens tbd-framework with clean script compile on my Linux/Steam install.

1. Confirm game vs Tools Steam build IDs match; tell me if I need to update.
2. Walk through base game path + adding tbd-framework/addon.gproj only.
3. Use enfusion-mcp to verify RestCallback, JsonLoadContext, and Resource.Load before changing any TBD_*.c file.
4. Fix any remaining compile errors in tbd-framework/ only — no Coalition code.

Do not open or depend on Tbd_framework/.
```

---

## Discord / ops (human)

- Milestone post draft: `docs/discord-milestone-1-post.md`
- Website announcement already seeded (Sat 22 Aug 2026)
