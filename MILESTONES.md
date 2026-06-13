# TBD Reforger Platform — Milestones

Every phase ends with a real event on real infrastructure (design pillar 5).
Dates below are **proposed targets** anchored to the plan's phase estimates from a
2026-06-13 start. A real, publicly posted date is the only scheduling tool that
works for community projects — confirm Milestone #1 in Discord and replace
"proposed" with "confirmed" once posted.

| Milestone | Proposed date | Phase | Gate |
|---|---|---|---|
| Milestone #1 — first manual event | **Sat 2026-08-22** (announced on site) | end of Phase 1 | mission loads from backend, slots enforce, a side wins, no Workbench |
| Milestone #2 — VOIP integration event | **Sat 2026-11-21** (proposed) | end of Phase 3 | bridge contract live, radio nets + dead channel working at 20+ players |

---

## Milestone #1 — First manual event

- **Date:** Saturday 2026-08-22 — **announced on the event website** (Discord: paste from [`docs/discord-milestone-1-post.md`](docs/discord-milestone-1-post.md)).
- **Scale:** 20-40 players, internal test.
- **Mission:** hand-written Mission JSON (e.g.
  [`tbd-schema/golden-missions/bridgehead-at-levie.json`](tbd-schema/golden-missions/bridgehead-at-levie.json)),
  served via `GET /api/missions/{id}/compiled`.
- **VOIP:** optional — in-game VON is an acceptable fallback. Not a gate.

### Success criteria

- [ ] Mission loads on the dedicated server from the backend REST endpoint
      (file fallback proven as backup).
- [ ] Slots enforce: players get the role they were assigned; no free-for-all.
- [ ] A side wins via a real win condition (capture / hold / elimination).
- [ ] Results POSTed back to the backend and visible on the event page.
- [ ] Nobody needed Workbench to play.

### Entry checklist (must be done before the date)

- [ ] Phase 0 spikes green: REST (0.1, done), registry (0.4), schema frozen (0.5).
- [ ] Framework MVP: state machine, loader, spawner, capture objective, loadouts,
      safe start, boundary, admin commands.
- [ ] Web: mission upload + validation, event linked to a mission, manual ORBAT
      assignment, identity linking.
- [ ] Staging soak on the pinned game/mod version; golden-mission smoke test passes.
- [ ] Event frozen 48 h in advance (no game/mod updates).

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
- This file is the scheduling source of truth. Update statuses (proposed ->
  confirmed -> done) as milestones land.
