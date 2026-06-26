# TBD Reforger Platform — Milestones

Every phase ends with a real event on real infrastructure (design pillar 5).
Dates below are **proposed targets** anchored to the plan's phase estimates from a
2026-06-13 start. Milestone #1 is **announced on the website**; post in Discord
from [`docs/discord-milestone-1-post.md`](docs/discord-milestone-1-post.md) when ready.

| Milestone | Proposed date | Phase | Gate |
|---|---|---|---|
| Milestone #1 — first manual event | **Sat 2026-08-22** (announced on site) | end of Phase 1 | mission loads from backend, slots enforce, a side wins, no Workbench |
| Milestone #2 — VOIP integration event | **Sat 2026-11-21** (proposed) | end of Phase 3 | bridge contract live, radio nets + dead channel working at 20+ players |

---

## Milestone #1 — First manual event

- **Date:** Saturday 2026-08-22 — announced on the event website.
- **Scale:** 20-40 players, internal test.
- **Mission:** hand-written Mission JSON (e.g.
  [`tbd-schema/golden-missions/bridgehead-at-levie.json`](tbd-schema/golden-missions/bridgehead-at-levie.json)),
  served via `GET /api/missions/{id}/compiled`.
- **VOIP:** optional — in-game VON is an acceptable fallback. Not a gate.

### Success criteria

- [x] Mission loads on the dedicated server from the backend REST endpoint
- [x] File fallback proven as backup (`$profile:missions/{id}.json`)
- [x] Per-slot spawn at mission JSON coords (staging dedicated server verified 2026-06-14)
- [x] Staging server deploy on 192.168.0.140 (`scripts/deploy-staging.sh`) — Workshop mod + `-config`, client Direct-Joined + spawned 2026-06-14
- [ ] Slots enforce: roster identity → assigned slot (round-robin only until linking)
- [ ] A side wins via a real win condition (capture / hold / elimination)
- [ ] Results POSTed back to the backend and visible on the event page
- [x] Nobody needed Workbench to play — players join the dedicated server via Direct Join (2026-06-14)

### Entry checklist (must be done before the date)

- [x] Phase 0 spikes green: REST (0.1), registry (0.4), schema frozen (0.5)
- [ ] Framework MVP: state machine, loader ✓, spawner ✓ (round-robin), capture objective, loadouts,
      safe start, boundary, admin commands
- [ ] Web: mission upload + validation (API ✓, UI pending), event linked to mission,
      manual ORBAT assignment (API ✓, UI pending), identity linking (API ✓, UI pending)
- [ ] Staging soak on the pinned game/mod version; golden-mission smoke test passes
- [ ] Event frozen 48 h in advance (no game/mod updates)
- [ ] Discord announcement posted (draft ready)

---

## Milestone #2 — VOIP integration event

- **Proposed date:** Saturday 2026-11-21 (after partner Phase 0.2 + Phase 3 integration).
- **Scale:** 20+ players.
- **Depends on:** partner VOIP capability matrix
  ([`tbd-schema/spikes/voip-capability-matrix.md`](tbd-schema/spikes/voip-capability-matrix.md))
  and the locked bridge contract
  ([`tbd-schema/bridge/bridge-contract.md`](tbd-schema/bridge/bridge-contract.md)).

### Success criteria

- [ ] Bridge contract version locked and implemented on both sides.
- [ ] Radio nets assigned from Mission JSON `radioPlan` on spawn.
- [ ] Push-to-talk works per net (command + squad independently).
- [ ] Dead channel isolation verified live (living players cannot hear the dead).
- [ ] Console strategy decision (A/B/C) applied and communicated to players.
- [ ] AAR replay link published within 24 h.

### Entry checklist

- [ ] Partner: TBD Voice client + server MVP and TBD-Radio bridge mod.
- [ ] Main team: framework radio hooks (`OnPlayerSpawned`, `OnPlayerKilled`,
      `OnRadioRetune`, `OnPTT`, `OnStageChanged`) wired to the bridge.
- [ ] Joint dry run on staging before the public event.

---

## Notes

- Dates assume an evenings-and-weekends pace; pull them in if anyone goes
  near-full-time.
- This file is the scheduling source of truth. Update checkboxes as criteria land.
