# TBD Event — Reforger Event Platform: Master Build Plan

**Version:** 1.0 draft · **Scope:** Mission framework (Enfusion), web platform (mission wizard + slotting + payments), VON enhancement mod, admin tooling, infrastructure
**Assumptions:** part-time community dev team, monetization approval already granted, console parity (Xbox/PS5) is a hard requirement, in-game VON only (no external voice client).

---

## 0. Design pillars

Every decision below traces back to five pillars. When in doubt, re-read these.

1. **Missions are data, not mods.** One framework scenario per terrain; every mission is a JSON document the server fetches at load. No Workbench, no workshop upload, no client re-download per mission. This is what makes the grandma test passable and console players first-class citizens. This is the *default* path — section 12 defines two escalation tiers for hand-crafted detail and fully custom missions.
2. **The server is the authority.** The game server pulls missions, enforces slots, applies loadouts, scores objectives, and reports results. Clients are never trusted with anything that matters.
3. **One contract, many consumers.** The Mission JSON schema is shared by the web wizard (writes it), the validation engine (checks it), the Enfusion framework (executes it), and the ORBAT/radio/slotting systems (derive from it). Changes to the schema are the most expensive changes in the project — version it from day one.
4. **License-clean by construction.** All runtime code is original work owned by TBD. The modset on monetized servers contains only: vanilla content, TBD-owned mods, and third-party mods with verified commercial permission. A license matrix is a living, mandatory document.
5. **Events are the QA harness.** Every phase ends with a real event on real infrastructure. Nothing is "done" until it has survived 60+ players on a Saturday.

---

## 1. System architecture

```
┌──────────────────────────── WEB PLATFORM (reforger.tbdevent.eu) ───────────────────────────┐
│  Next.js front-end          API (REST/JSON)            Postgres          Object storage    │
│  ├ Mission Wizard (map UI)  ├ /missions /events        ├ users/identity  ├ mission JSON    │
│  ├ Event pages + slotting   ├ /roster /entitlements    ├ orbats/slots    ├ AAR recordings  │
│  ├ Payments (Stripe)        ├ /telemetry /results      ├ payments        ├ map tiles       │
│  └ Live ops dashboard       └ /broadcast (admin)       └ license matrix  └ registry files  │
└───────────────▲─────────────────────────▲──────────────────────────────────────────────────┘
                │ OAuth (Discord)         │ HTTPS, token-authed (server credentials)
                │ identity linking        │
┌───────────────┴──────────┐   ┌──────────┴───────────────────────────────────────────────────┐
│  PLAYERS                 │   │  GAME SERVERS (Linux, dockerised)                             │
│  PC / Xbox / PS5         │   │  Arma Reforger dedicated server                               │
│  download mods from      │◄──┤  ├ TBD-Framework mod (game mode, loader, spawner, slots,     │
│  Reforger Workshop;      │   │  │   objectives, telemetry, admin)                            │
│  join server; no other   │   │  ├ TBD-VON mod (radios, nets, dead channel)                  │
│  software required       │   │  └ TBD-Content mod (compositions, loadout assets)            │
└──────────────────────────┘   └──────────────────────────────────────────────────────────────┘
```

**Key data flows**

- **Mission publish → play:** Wizard saves Mission JSON → validation passes → stored with version hash → admin schedules event → game server starts with `missionId` in its config → TBD-Framework calls `GET /api/missions/{id}/compiled` at scenario load → spawner builds the world.
- **Slot enforcement:** Player joins lobby → framework reads their platform identity → `GET /api/events/{id}/roster` → player sees only the slot they claimed on the site (plus open slots if the event allows walk-ins).
- **Telemetry:** Framework batches gameplay events (kills, captures, ticket changes, position samples) → `POST /api/telemetry` every N seconds → powers live dashboard and post-event AAR replay.
- **Results:** Mission end → framework posts final score/winner/stats → event page updates automatically.
- **Identity linking (one-time per player):** Site shows "link your game account" → player joins any TBD server → framework displays a 6-digit code in the lobby UI → player enters it on the site → backend binds site account ↔ Reforger identity ID.

**Why HTTP and not files:** the dedicated server runs Enfusion script with REST capability, which lets the loop stay closed (pull missions, push results) with one integration style. Keep a file-based fallback (`$profile/missions/{id}.json`) for offline testing and as insurance — the loader should accept either source behind one interface. *Spike task 0.1 verifies the REST path on a dedicated Linux server before anything else is built on it.*

---

## 2. The contract: Mission JSON schema v1

This is the most important artifact in the project. It is consumed by four systems and authored by non-technical users through the wizard. Rules:

- Published missions are **immutable**; edits create a new version (content-hash IDs). The server logs which version it ran.
- The schema ships as a formal JSON Schema file in a shared repo; the web validator and the Enfusion loader both treat it as the source of truth.
- The wizard never emits raw prefab paths. It emits **registry aliases** (`"kit:us_rifleman_82"`, `"comp:checkpoint_small"`, `"veh:m151_mg"`) that the framework resolves through a registry file (section 6). This decouples the web from engine internals and lets you swap assets without touching missions.

### 2.1 Schema outline with example

```jsonc
{
  "schemaVersion": "1.0",
  "meta": {
    "id": "msn_8f3a2c",            // content-hash assigned on publish
    "name": "Bridgehead at Levie",
    "author": "grandma@tbdevent.eu",
    "terrain": "everon",            // must match a framework scenario
    "templateId": "attack_defend_v1",
    "playerRange": [40, 96]
  },
  "environment": {
    "dateTime": "1989-06-14T05:30:00",
    "weatherPreset": "overcast_light",
    "windDirDeg": 240
  },
  "factions": [
    {
      "key": "blufor",
      "displayName": "US Army",
      "presetId": "preset:us_army_82nd",   // resolves faction, uniforms, default kits
      "tickets": 0,                          // 0 = one-life, no respawn pool
      "color": "#2b6cb0"
    },
    { "key": "opfor", "displayName": "Soviet VDV", "presetId": "preset:sov_vdv", "tickets": 0, "color": "#c53030" }
  ],
  "orbat": {
    "blufor": {
      "groups": [
        {
          "callsign": "Alpha", "type": "rifle_squad",
          "roles": [
            { "slot": "SL",  "kit": "kit:us_sl",        "count": 1, "radio": ["net:cmd", "net:alpha"] },
            { "slot": "TL",  "kit": "kit:us_tl",        "count": 2, "radio": ["net:alpha"] },
            { "slot": "AR",  "kit": "kit:us_ar",        "count": 2, "radio": [] },
            { "slot": "RFL", "kit": "kit:us_rifleman",  "count": 4, "radio": [] }
          ]
        }
        // ... more groups; wizard auto-generates from player-count sliders
      ]
    },
    "opfor": { "groups": [ /* ... */ ] }
  },
  "radioPlan": {
    "nets": [
      { "id": "net:cmd",   "label": "BLUFOR Command", "freqMHz": 41.0, "faction": "blufor" },
      { "id": "net:alpha", "label": "Alpha Squad",    "freqMHz": 42.5, "faction": "blufor" }
    ]
  },
  "zones": [
    { "id": "z1", "type": "spawn",            "faction": "blufor", "shape": { "circle": { "x": 4831.2, "z": 6620.8, "r": 150 } } },
    { "id": "z2", "type": "spawn",            "faction": "opfor",  "shape": { "circle": { "x": 6010.0, "z": 7211.5, "r": 150 } } },
    { "id": "z3", "type": "objective_capture", "label": "Levie Bridge",
      "shape": { "circle": { "x": 5402.0, "z": 6890.0, "r": 80 } },
      "rules": { "captureSeconds": 120, "contestable": true, "points": 100 } },
    { "id": "z4", "type": "boundary", "shape": { "polygon": [[4500,6400],[6400,6400],[6400,7600],[4500,7600]] },
      "rules": { "graceSeconds": 30, "penalty": "kill" } },
    { "id": "z5", "type": "base_protection", "faction": "opfor", "shape": { "circle": { "x": 6010.0, "z": 7211.5, "r": 250 } } }
  ],
  "entities": [
    { "alias": "veh:m151_mg",          "x": 4840.1, "z": 6633.0, "headingDeg": 90, "faction": "blufor" },
    { "alias": "comp:checkpoint_small", "x": 5380.5, "z": 6901.2, "headingDeg": 184 }
  ],
  "layers": ["layer:levie_defenses_v2"],   // hand-built decoration layers, see §12 Tier 2
  "flow": {
    "briefingSeconds": 600,
    "safeStartSeconds": 300,
    "timeLimitSeconds": 5400,
    "jip": "until_safestart_end"      // join-in-progress policy
  },
  "winConditions": {
    "mode": "points_then_attrition",
    "endOn": ["time_limit", "all_objectives_captured", "faction_eliminated"]
  },
  "briefings": {
    "blufor": { "situation": "…", "mission": "…", "execution": "…", "markers": [ { "x": 5402, "z": 6890, "icon": "objective", "label": "OBJ BRIDGE" } ] },
    "opfor":  { "situation": "…", "mission": "…", "execution": "…", "markers": [] }
  },
  "settings": { "respawn": "none", "spectatorPolicy": "own_side_delayed_60s", "nightVision": false }
}
```

### 2.2 Schema governance

- Changes go through a lightweight RFC in the shared repo; the framework advertises the schema versions it supports, and the backend refuses to serve a mission to a server that can't run it.
- Write a **compatibility test**: a folder of golden mission files that must always validate and load; run it in CI for both the web validator and (manually, pre-release) the Enfusion loader.

---

## 3. Workstream A — TBD-Framework (Enfusion runtime)

The framework is one workshop mod containing a generic game mode plus per-terrain scenario configs (`.conf`) that all point at the same systems. Written entirely in Enfusion script; no heavy assets here so the mod stays small for console downloads.

> **API caution:** Enfusion script classes referenced below (`SCR_BaseGameMode`, `SCR_JsonLoadContext`, `RestApi`/`RestContext`, trigger entities, `BackendApi` identity calls) evolve between game versions. Treat every named class as "verify against current BI wiki and sample projects during the spike," not gospel. Bohemia's official Scenario Framework and sample repos are legitimate references for *patterns* — your implementation must be your own code.

### A1. Game mode shell & state machine

- Subclass the base game mode; implement a replicated state machine: `LOADING → LOBBY → BRIEFING → SAFE_START → LIVE → END → DEBRIEF`.
- Each state owns: allowed player actions (weapons safe? movement locked?), UI shown, timers, and transition conditions (admin command, timer expiry, or win condition).
- All transitions happen on the authority (server) and replicate to clients; clients render state, never decide it.
- Build a tiny **admin command bus** early (chat-command or admin menu driven): `#stage next`, `#stage set LIVE`, `#end blufor` — events will need manual overrides, guaranteed.

### A2. Mission loader

- Read order: (1) `missionId` from server config/startup parameter → (2) `GET {backend}/api/missions/{id}/compiled` with a per-server bearer token → (3) on failure, fall back to `$profile/missions/{id}.json` → (4) on failure, refuse to leave `LOADING` and print a loud error for admins.
- Parse into strongly-typed mission model classes (one class per schema section). Validate hard: unknown alias, zone outside terrain bounds, role counts ≤ 0 → abort with explicit log lines, never "best effort" — a half-spawned event mission is worse than no mission.
- Cache the fetched JSON to `$profile` with its hash so a mid-event server restart can reload identically even if the backend is down.

### A3. Runtime spawner

- **Registry resolution:** load `registry.json` (shipped inside the mod, generated by the Workbench export tool — section 6) mapping aliases → prefab resource GUIDs. Missions never contain GUIDs.
- **Spawn pipeline (during `LOADING`):** factions → ORBAT groups & spawn points → entities/vehicles (terrain-snapped, heading applied) → compositions (multi-prefab sets with relative transforms) → zone triggers (circle = trigger entity radius; polygon = polyline area) → markers.
- Spawn in batches across frames to avoid a multi-second server hitch; log a spawn manifest (alias → entity ID) for the admin tools and cleanup.
- **Compositions:** author them as prefab hierarchies in TBD-Content with a defined origin/footprint, so the web map can show an accurate footprint circle/box while grandma places them.

### A4. Slots, identity & ORBAT enforcement

- On player connect, obtain the platform identity ID, then check it against the event roster (`GET /api/events/{id}/roster`, cached and refreshed periodically).
- Lobby UI lists ORBAT groups/roles from the mission JSON. Behavior per role: **claimed** (only the matching identity can take it), **open** (walk-in allowed if the event permits), **locked** (admin only).
- Unclaimed no-shows: configurable release timer (e.g., role unlocks 10 min after briefing starts) so events don't run short-handed.
- Spectator is a first-class role with its own slot type, used by casters and admins.

### A5. Loadout system

- On spawn, apply the role's kit from the JSON: clear default inventory → equip uniform/vest/backpack → weapons with attachments → magazines/items by count. All via alias resolution.
- Kits are defined once in the registry (per preset), missions reference them; per-mission overrides allowed in an "advanced" JSON block the wizard hides by default.
- Lock arsenals/equipment boxes unless the mission explicitly spawns one (supply gameplay should be a deliberate mission feature, not a leak).

### A6. Objectives & scoring

- v1 objective types: `objective_capture` (presence-based with contest rules and capture timer), `objective_destroy` (bind to spawned target entities' destruction events), `objective_hold_until` (defender scores by surviving the clock). These three cover the classic WOG/TBD attack-defend repertoire; add `intel_grab`/`escort` later.
- A replicated score/ticket manager owns the numbers; an end-condition evaluator checks `winConditions` every few seconds during `LIVE`.
- Faction-elimination detection must respect unconsciousness/medical states so a mission doesn't end while half a squad is revivable.

### A7. Safety & fairness systems

- **Safe start:** during `BRIEFING`/`SAFE_START`, disable damage dealing (intercept damage events on the authority) and optionally lock vehicles to drivers' own faction.
- **Boundary zones:** warning UI + grace timer → kill (logged). **Base protection:** entering enemy base protection during configured windows = warning → kill.
- **One-life enforcement:** on death, player transitions to the dead/spectator flow (A9); no respawn unless mission JSON says otherwise (the schema supports tickets for future modes).

### A8. Briefing, markers & in-game UI

- Briefing screens per faction during `LOBBY`/`BRIEFING`, rendering the JSON briefing sections and map markers (own side only).
- Every screen must be **fully gamepad-navigable** — treat controller UX as a release gate, not a nice-to-have. Test on an actual Xbox controller from day one of UI work.
- Markers render on the in-game map; enemy markers never replicate to the other faction (replication filtering on the authority).

### A9. Death, spectating & anti-ghost policy

- On death: configurable per mission — `spectatorPolicy` of `none` (black screen + dead chat), `own_side_delayed_60s` (recommended default: follow-cam on own faction with a delay), or `free` (for casual events).
- All spectating camera logic is server-gated; the dead never receive enemy position replication beyond what policy allows.
- Hooks into TBD-VON: on death the player's VON capability switches to the dead channel (Workstream C).

### A10. Telemetry, results & AAR capture

- Event stream: kills (attacker/victim/weapon/distance), zone capture progress, ticket/score changes, stage transitions, admin actions.
- Position sampling for AAR replay: every entity of interest at 1–2 s resolution, batched and POSTed (or buffered to `$profile` and uploaded at mission end if bandwidth is a concern). Budget check in the spike: 128 players × 2 h × 0.5 Hz ≈ 460k samples — trivial as compressed JSON/NDJSON, but confirm server frame cost of collection.
- On `END`: POST final results (winner, score timeline, per-player stats) → backend marks the event complete and publishes the AAR page.

### A11. In-game admin tools

- Admin menu (permission-checked against backend roster role): stage control, broadcast message (text + optional sound cue), teleport player, kick/move to spectator, end mission with winner.
- Web-triggered actions: the framework polls `GET /api/servers/{id}/commands` (or the broadcast endpoint) every few seconds during events so the live ops dashboard can act without anyone alt-tabbing.

### A12. Engineering practices for the mod

- Git repo with the standard Enfusion addon layout; PR review mandatory for the two-scripter core team; changelog per workshop release.
- **Three workshop channels:** `TBD-Framework-Dev` (volatile), `-Staging` (release candidates soaked on the staging server), `-Release` (what events run). Same for VON and Content mods.
- After every Reforger game update: staging soak with a scripted smoke test mission before any production event. Pin event nights to known-good game/mod version pairs.
- Console compatibility checklist per release: no PC-only APIs, gamepad nav verified, mod size budget respected, tested by at least one Xbox and one PS5 community member.

---

## 4. Workstream B — Web platform (reforger.tbdevent.eu)

**Recommended stack:** Next.js + TypeScript, Postgres (Prisma or Drizzle), Discord OAuth for accounts, MapLibre GL or Leaflet for the map UI, Stripe for payments, S3-compatible object storage, deployed on a EU VPS or Hetzner + standard CI/CD. Nothing exotic — boring technology wins here because the Enfusion side will consume all your novelty budget.

### B1. Accounts & identity linking

- Discord OAuth as the primary login (the community already lives there). Store: Discord ID, display name, roles (admin/mission-maker/player/supporter).
- **Game identity link (one-time):** site issues a short-lived 6-digit code → player joins any TBD server and opens "Link account" in the lobby → enters code → framework POSTs `{code, identityId, platform}` → backend binds them. Handles PC, Xbox, and PS5 with one flow and zero platform-specific OAuth pain.
- One site account can hold multiple game identities (people who play on two platforms).

### B2. Mission Wizard — the grandma path

This is the product. Budget UI/UX time accordingly.

- **Map viewer:** tiled 2D maps per terrain. Tile pipeline (spike 0.3): produce a top-down ortho of each terrain (Workbench export or high-altitude captures), slice with `gdal2tiles`, store per terrain+version in object storage. Calibrate pixel→world coordinates once per terrain and store the transform with the tileset.
- **Wizard flow (linear, one decision per screen):**
  1. Pick template → "Attack / Defend" (v1's only template).
  2. Pick terrain → map loads.
  3. Draw zones → guided: "place the defender's area" (circle tool), "place attacker spawn," "draw the play boundary" (rectangle/polygon). Each tool enforces constraints live (min/max radius, minimum spacing) with friendly errors.
  4. Pick factions & era from preset cards with pictures.
  5. Set player counts with two sliders (per side, with ratio presets like 2:1 Atk:Def) → ORBAT auto-generates; an "adjust squads" accordion exists but is optional.
  6. Briefing → three plain-text boxes per side with placeholder guidance text.
  7. Environment → time-of-day slider with a day/night preview strip, weather preset cards.
  8. Review → validation report, estimated mission stats (walk distances between zones, slot totals) → **Publish**.
- **Validation engine:** shared TypeScript package implementing the schema plus gameplay lint rules (spawns inside boundary, objective not inside a spawn, walk distance sanity, radio plan completeness — auto-generate the radio plan from the ORBAT so grandma never sees a frequency).
- **Power-user mode:** raw JSON editor with schema autocomplete behind a "developer mode" toggle, plus mission cloning and versioned history. This is your escape hatch instead of Workbench for 90% of advanced needs.

### B3. Events & slotting

- Create event from a published mission: date/time, capacity, visibility (public / supporters-only / unlisted), walk-in policy, sign-up open/close times.
- ORBAT slotting page mirroring the mission's structure: tap a role to claim it; leaders can be granted slot-management rights for their group; waitlist with auto-promotion; check-in window before the event with auto-release of no-show claims.
- Discord integration: event announcements, slot-claim confirmations, T-60/T-10 reminders, and a post-event AAR link via a small bot or webhooks.

### B4. Payments & entitlements (inside Bohemia's rules)

- Stripe products: **supporter subscription** (cosmetic/priority-neutral perks: name flair on site, early sign-up window for *non-gameplay* slots is risky — keep perks strictly off-gameplay: site flair, AAR video archive access, supporter-only *social* events) and **server access fees** for designated supporter-only event nights (allowed: limiting access to paying players).
- Entitlement API: `GET /api/entitlements/{identityId}` → framework checks at join for supporter-only events.
- Bookkeeping: Stripe Tax for EU VAT, invoices, and a visible terms page. Revisit perk design against Bohemia's current published rules each time you add one, and calendar the re-application before the published permission window expires (currently stated as Jan 31, 2027 — verify periodically).

### B5. Live ops dashboard

- Per-server card: online/offline, game version, mod versions, current mission + stage, player count vs roster, last telemetry heartbeat.
- Live actions: broadcast message, stage override request, kick/move (all delivered via the command-polling endpoint the framework already implements).
- Telemetry ticker during events (captures, kills, score) — this doubles as the casters' screen.

### B6. AAR module

- 2D replay player: same map tiles, scrub bar, faction-colored dots from position samples, kill feed overlay, zone-state timeline. Export of score graphs for the community write-ups.
- This is the feature that replaces the voice recording you gave up with in-game VON — make replays shareable (public URL per event) because they're also your best marketing asset.

### B7. API surface (initial)

| Endpoint | Auth | Purpose |
|---|---|---|
| `GET /api/missions/{id}/compiled` | server token | Mission JSON for the loader |
| `GET /api/events/{id}/roster` | server token | identity→slot map, admin roles, walk-in policy |
| `POST /api/link` | server token | bind 6-digit code → game identity |
| `GET /api/entitlements/{identityId}` | server token | supporter checks |
| `POST /api/telemetry` | server token | batched gameplay events / position samples |
| `POST /api/results` | server token | final mission results |
| `GET /api/servers/{id}/commands` | server token | queued admin actions (poll) |
| `POST /api/missions` (+validate) | user session | wizard publish path |
| `POST /api/events/{id}/slots/{slotId}/claim` | user session | slotting |

---

## 5. Workstream C — TBD-VON (in-game voice enhancement)

**Scope guardrail:** build *on top of* the engine's VON. Anything requiring new native audio behavior is out of scope; discover the hard limits in a spike before promising features to the community.

- **C1. Radio items:** handheld (squad net), manpack (long-range/command), vehicle radios — TBD-owned models or vanilla radios re-configured. Range and channel behavior driven by transceiver configuration.
- **C2. Frequency & net system:** net presets injected from the mission JSON `radioPlan` on spawn — a squad leader spawns already tuned to `cmd` + own squad net. Manual tuning UI (gamepad-first: d-pad steps, hold-to-scan) for in-mission re-tasking and "espionage" play.
- **C3. Dead/spectator channel:** on death, swap the player's VON capability to a dead-only channel; living players can never hear it. Admin/caster roles may listen to any net (read-only) for casting.
- **C4. Admin broadcast:** server-triggered voice-channel override is likely engine-restricted — implement admin broadcast as prominent UI + sound cue instead, and treat voice-override as a stretch goal pending the spike.
- **C5. Feasibility spike (must run in Phase 0):** verify, on the current game version: per-net audibility control from script, dead-channel isolation, transceiver range/frequency configurability, and any stereo/ear-assignment hooks. Output: a one-page "VON capability matrix" that fixes C-scope for v1. Do not skip this — VON internals are the least scriptable part of the engine and this spike protects the roadmap.

---

## 6. Workstream D — TBD-Content & the registry

- **Compositions library:** FOBs, checkpoints, trench sets, camps, roadblocks — authored once in Workbench as prefab hierarchies with documented origins/footprints. Each gets a registry alias, a thumbnail, and a footprint definition the wizard renders on the map.
- **Faction/era presets:** start with vanilla US/Soviet 1989; add preset packs only from license-cleared sources or TBD-made content.
- **Registry export tool:** a small Workbench plugin/script that walks tagged prefabs and emits `registry.json` (alias → GUID + metadata) plus `registry-web.json` (alias → display name, thumbnail, footprint) for the wizard. Run on every Content release; CI check that every alias used by golden missions still resolves.

---

## 7. Licensing & compliance track (continuous)

1. **License matrix** (a sheet in the repo, reviewed before any modset change): mod name, workshop ID, license, commercial-use status (`yes / no / written-permission-on-file / unknown=no`), permission document link, last verified date.
2. **Monetized-server policy:** modset = vanilla + TBD mods + `written-permission-on-file` mods only. Anything else (RHS-class EULAs especially) runs only on free community nights on separate, non-monetized servers — or not at all, to keep the story simple.
3. **Own-mod licensing:** publish TBD mods under a restrictive license of your choice (e.g., all rights reserved with explicit "no reuse without permission") — you own them, you decide. Document copyright headers in every script file.
4. **Clean-room discipline:** contributors may read CRF/TilW/BI samples for *patterns*; copying script verbatim or near-verbatim is prohibited and checked in PR review. Keep a short CONTRIBUTING.md stating this so it's provable policy.
5. **Bohemia program hygiene:** keep the approval listing current, re-read the rules whenever adding a perk, calendar the permission-window renewal, and screenshot/archive the rules version you operated under.

---

## 8. Infrastructure & DevOps

- **Game servers:** Linux dedicated hosts (the Reforger Linux server is standard), containerised (Docker; Pterodactyl panel optional). Config templating injects `missionId`, backend URL, and the server token per instance. Separate **dev / staging / prod** instances mapped to the three workshop channels.
- **Sizing:** start with one strong prod box (high single-core performance matters for Reforger) + one small staging box; add a second prod server only when event demand forces parallel sessions.
- **Update discipline:** game updates land on staging first; smoke-test mission (a golden JSON that exercises spawner, zones, loadouts, telemetry) must pass before prod upgrades. Event nights are frozen 48 h in advance.
- **Web:** CI/CD with preview deploys, nightly Postgres backups, object-storage lifecycle rules for telemetry, uptime monitoring with alerts to the admin Discord channel.
- **Secrets:** per-server bearer tokens, rotated; Stripe keys in the platform secret store; no credentials in mods or mission files, ever.

---

## 9. Team, phases & milestones

**Minimum viable team:** 1–2 Enfusion scripters (A + C), 1 full-stack web dev (B), 1 person on content/Workbench (D) who can be one of the scripters part-time, 1 producer/mission-design lead who owns the schema and runs events, plus community testers (must include one Xbox and one PS5 player). Estimates below assume evenings-and-weekends pace; halve them if anyone goes near-full-time.

**Phase 0 — Spikes & contracts (2–3 weeks).** Decisions before code.
- 0.1 REST spike: dedicated Linux server fetches JSON from the backend and POSTs a result. *Go/no-go for the whole architecture; file-based fallback is the contingency.*
- 0.2 VON capability spike → C-scope matrix (section C5).
- 0.3 Map-tile pipeline spike for Everon, including the coordinate transform.
- 0.4 Registry export proof-of-concept (five aliases end to end).
- 0.5 Schema v1 frozen and golden missions written by hand.
- **Milestone:** architecture review; all spikes green or contingencies chosen.

**Phase 1 — Framework MVP + manual web (6–8 weeks).**
- Enfusion: game-mode shell, loader (REST + file), spawner, capture objective, loadouts, safe start, boundary, end screen, basic admin commands.
- Web: auth + identity linking, hand-written JSON upload with validation, event page with manual slotting list.
- **Milestone event #1:** 20–40 player internal test on a hand-written mission. Success = mission loads from the backend, slots enforce, a side wins, nobody needed Workbench.

**Phase 2 — The wizard + enforcement (6–8 weeks).**
- Wizard v1 (one template, Everon, circle/polygon tools, auto-ORBAT, validation), slot claiming UI, in-game roster enforcement, telemetry + results, Discord bot.
- **Milestone event #2:** first public event whose mission was built entirely in the wizard — ideally by a genuinely non-technical community member. This is the grandma test, run literally.

**Phase 3 — VON + AAR + money (6–8 weeks).**
- TBD-VON v1 per the capability matrix (radios, nets-from-JSON, dead channel), AAR replay v1, live ops dashboard, Stripe supporter tier + entitlement checks, and Tier 2 authoring (in-game build-mode layer capture + wizard layer attach, §12).
- **Milestone event #3:** full-stack event night — paid supporter early access (within the rules), wizard mission, VON nets, published AAR replay link within 24 h.

**Phase 4 — Scale & product (ongoing).** Second terrain, second template (meeting engagement), composition library growth, Mission Maker SDK + first flagship Tier 3 custom mission (§12), mission library with ratings, white-label exploration for other communities, second prod server.

---

## 10. Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| VON less scriptable than hoped | Medium | High | Phase 0 spike fixes scope before promises; fallback = simpler net model + UI-based broadcast |
| Server REST path blocked/unreliable | Low–Med | High | File-based loader fallback already designed in; a tiny host-side sync daemon can ferry files if needed |
| Game update breaks mods before an event | High (eventually) | High | Staging soak, version pinning, 48 h freeze, golden-mission smoke test |
| Console compatibility regressions | Medium | High | Console checklist per release; Xbox+PS5 testers in every milestone event |
| Schema churn breaks old missions | Medium | Medium | Versioned schema, golden-mission CI, server advertises supported versions |
| License slip on a monetized server | Low | Severe | License matrix gate on modset changes; vanilla-first content policy |
| Monetization rules change / window lapses | Medium | High | Calendar renewal; donations-only fallback plan; archive rule versions |
| Burnout / bus factor | High | High | Two people minimally familiar with each system; docs in-repo; phase milestones are real events that keep morale visible |
| Tier 3 custom mission fails late or misses the download window | Medium | High | §12 governance gates; 72 h publish rule; staging dry run; wizard-built fallback mission always on the event schedule |
| Tile/coordinate drift after terrain updates | Medium | Low | Tilesets versioned per terrain version; calibration test mission |

---

## 11. First 14 days — concrete checklist

1. Create the GitHub org + repos: `tbd-framework`, `tbd-von`, `tbd-content`, `tbd-web`, `tbd-schema` (schema, golden missions, registry format, CONTRIBUTING with clean-room policy).
2. Publish placeholder Dev workshop items for the three mods to lock names/IDs.
3. Write Workbench environment setup doc; both scripters reproduce it.
4. Stand up the staging Linux server + a skeleton backend endpoint serving one static mission JSON.
5. Run spikes 0.1–0.4; record outcomes as one-pagers in the repo.
6. Freeze schema v1 with the whole team in one review call; hand-write two golden missions.
7. Start the license matrix with the intended event modset (expect it to be nearly empty — that's correct).
8. **Set the date for Milestone Event #1 publicly.** A real date is the only scheduling tool that works in community projects.

---

## 12. Authoring tiers — from grandma to master craftsman

Highly detailed custom missions are supported through three tiers that share **one runtime**: the framework always runs the show, so slotting, ORBAT, radio plans, safe start, scoring, telemetry, and AAR replays work identically regardless of how a mission was authored.

### Tier 1 — Wizard missions (data-only)

The default path as designed in section B2. Zero downloads, instant hotfixes, console-equal.

### Tier 2 — Decoration layers (detailed, still data-only)

Hand-placed detail *without* creating a mod. A **layer** is a JSON file of entity placements (registry alias + position/heading, nested compositions allowed) stored on the backend, versioned, and attachable to any mission via the schema's `layers` field. The spawner treats it as an extension of the `entities` block. Two ways to author one:

- **12.1 In-game build mode (recommended).** Run "build sessions" on the staging server: the mission detailer uses Game Master (or a trimmed TBD build mode) to dress the area live — trenches, wrecks, camps, props — then an admin command exports every placed entity to a layer JSON. Load the mission with the layer attached, walk it in first person, tweak, re-export. Because this happens in-game, **a detail-builder doesn't even need a PC** — a console player can be your best set-dresser. This workflow is faster than Workbench for set dressing and should be the celebrated path for "highly detailed."
- **12.2 Workbench layer export.** Power users place entities in World Editor in a throwaway world, then run the TBD export plugin (same tool family as the registry exporter) to emit the identical layer format.

Export-time validation: every alias must resolve against the event modset's registry; an entity-count/performance budget per layer; no terrain edits, no scripts, no new assets — those needs escalate to Tier 3. Layers are reusable and composable ("Levie defenses v2" + "Levie civilian clutter"), building a community library of detail work over time.

### Tier 3 — Full custom mission mods (framework-as-SDK)

For missions needing custom scripting, new mechanics, bespoke assets, or terrain edits, the framework doubles as a dependency:

- The custom mission is a workshop mod that **depends on TBD-Framework**, includes the framework game-mode prefab in its world, and binds to a backend `missionId` — so ORBAT, slotting, radio plan, flow timers, and telemetry still come from JSON, while the world and special mechanics are hand-built.
- **Mission Maker SDK** (documented, with a sample mission repo): a custom-objective interface that reports into the score manager; stage hooks (`OnBriefing`, `OnLive`, `OnEnd`); a telemetry-emit API so custom events appear in AAR replays; marker/briefing APIs.
- **Governance gates — mandatory, because these run on monetized servers:** code and license review (any new asset or mod dependency enters the license matrix before anything else happens); console-compat checklist; staging soak including one full dry run with testers; workshop publish at least **72 hours before the event** so all platforms pre-download; version pin through event night; a wizard-built fallback mission scheduled in case the custom one fails review or soak.
- Honest cost accounting: Tier 3 reintroduces everything Tier 1 removed — downloads, lead time, review burden, breakage risk. Reserve it for flagship events, and push every request that's really "more detail" rather than "new mechanics" down into Tier 2.

### Platform integration

The mission library tags each mission with its tier; event pages for Tier 3 missions show a "download required" badge with the workshop link and publish countdown; the wizard gains a "Layers" step where builders browse the layer library with footprint previews and attach/detach them. Roadmap placement: Tier 2 (build-mode capture, layer attach UI) lands in **Phase 3**; the SDK and the first flagship Tier 3 mission anchor **Phase 4**.

---

## Appendix A — Glossary

- **Registry / alias:** human-readable key (`kit:us_rifleman`) mapped to engine prefab GUIDs by a generated file; the only vocabulary missions and the wizard speak.
- **Golden missions:** hand-maintained mission JSONs that must always validate and load; the compatibility test suite.
- **Workshop channels:** Dev/Staging/Release copies of each mod for safe iteration.
- **Safe start:** post-briefing window where damage is disabled while players form up.
- **Walk-in:** a player without a claimed slot taking an open role at event start.
