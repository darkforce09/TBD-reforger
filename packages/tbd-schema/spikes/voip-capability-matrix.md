# VOIP Capability Matrix (Phase 0.2 output)

> **Status:** TEMPLATE — to be filled by the partner after the Phase 0.2 spike.
> Until this is completed, VOIP features are unconfirmed and Milestone #1 runs on
> in-game VON fallback.

## Transport decision

- **Chosen transport (game to client):** _TBD_
- **Chosen audio stack (client to server):** _TBD_
- **Rationale:** _TBD_
- **Measured mouth-to-ear latency @ 60 players:** _TBD ms_

## Feature matrix

| Feature | v1 target (plan) | Spike result | Notes |
|---|---|---|---|
| Multiple radio nets per role | Yes | _yes / no / stretch_ | |
| Manual frequency retune in-mission | Yes | _._ | |
| Long vs short range radios | Yes | _._ | |
| Direct/proximity voice | Spike decides | _._ | client-positional vs in-game VON |
| Dead channel on death | Yes | _._ | must prove isolation |
| Multi-net independent PTT | Yes | _._ | command + squad simultaneously |
| Encrypted/scrambled nets | Stretch | _._ | |
| Console support | Spike decides | _._ | see decision below |

## Console decision

- **Recommendation (A / B / C):** _TBD_
  - A. PC full VOIP; console on in-game VON fallback
  - B. Companion phone/tablet app for console
  - C. Defer console events until VOIP scope solved
- **Evidence:** _TBD_

## Bridge contract changes requested

- _List any proposed edits to `../bridge/bridge-contract.md`._

## Sign-off

- [ ] Partner: capability matrix complete
- [ ] Main team: bridge contract reviewed
- [ ] Both: bridge contract version locked in `tbd-schema`
