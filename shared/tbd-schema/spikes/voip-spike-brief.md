# Phase 0.2 — VOIP Architecture Spike (partner brief)

> **Owner:** Partner (VOIP track). **Runs in parallel** with main-team Phase 0.1
> (REST) and 0.5 (schema). This brief is the main team's input so the partner can
> prototype against realistic mission data before the bridge contract is locked.

## Goal

Decide the architecture for a TBD-owned, TFAR-like external voice stack
(Teamspeak-like client + voice server + in-game bridge mod) and produce a
**VOIP capability matrix** that fixes v1 scope. No production code required —
the output is a decision document plus a throwaway proof-of-concept.

## What the main team provides

- **Mission JSON `radioPlan`** — see [`../schema/mission.schema.json`](../schema/mission.schema.json)
  (`$defs.radioPlan` / `$defs.net`). Golden missions
  [`bridgehead-at-levie.json`](../golden-missions/bridgehead-at-levie.json) and
  [`last-stand-at-montfort.json`](../golden-missions/last-stand-at-montfort.json)
  carry realistic net layouts (command + per-squad nets, short/long range).
- **Draft bridge contract** — [`../bridge/bridge-contract.md`](../bridge/bridge-contract.md)
  and [`../bridge/bridge-messages.schema.json`](../bridge/bridge-messages.schema.json),
  with samples in [`../bridge/samples/`](../bridge/samples/). Prototype against
  these messages; propose changes back.
- **Framework hook points** — `OnPlayerSpawned`, `OnPlayerKilled`, `OnRadioRetune`,
  `OnPTT`, `OnStageChanged` (intent documented in the bridge contract; the main
  team verifies exact Enfusion symbols via Enfusion MCP).

## Questions to answer (must, before promising features)

1. **Transport:** How does the Enfusion mod reach the voice client? Evaluate
   local REST, WebSocket, named pipe, and shared memory. Study CVON/TFAR bridge
   patterns via Enfusion MCP `game_read`; implement clean-room.
2. **Latency:** End-to-end mouth-to-ear budget at 60+ concurrent players. Measure,
   do not guess.
3. **Multi-net PTT:** Can one client instance serve multiple radio nets with
   independent push-to-talk (command + squad simultaneously)?
4. **Console path:** Is PC-only VOIP with console players on direct in-game VON
   fallback acceptable for v1, or is console VOIP a hard blocker? (Desktop apps do
   not run on Xbox/PS5 today.) Recommend option A, B, or C from the plan's "Open
   decisions".
5. **Dead-channel isolation:** Prove living players cannot hear the dead channel.
6. **Transport stack choice:** WebRTC + custom SFU vs Mumble-compatible vs custom
   UDP. Prefer boring, battle-tested audio transport over novelty.

## Deliverables

- **VOIP capability matrix** (feature -> v1 yes/no/stretch, with evidence) —
  fill in [`voip-capability-matrix.md`](voip-capability-matrix.md).
- **Chosen transport stack** with a one-paragraph rationale.
- **Proposed edits** to the bridge contract (if any), raised against this repo.
- Throwaway PoC proving transport + dead-channel isolation (link or repo).

## Definition of done

The capability matrix is filled, the console decision is made with evidence, and
the main team + partner jointly lock the bridge contract version in this repo.
