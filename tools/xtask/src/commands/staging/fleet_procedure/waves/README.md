# Fleet waves

The steps of the fleet procedure's waves W1–W14: stop, start and restart, the console command and
the player listing, the three mission deployments, then on one server each the identity link, the
kicks, the two credential rotations and the two lost acknowledgements, each built from one row of
the wave table and run by the staging procedure runner.

## Contents

```text
tools/xtask/src/commands/staging/fleet_procedure/waves/
├── console_waves.rs               W4 console `#players` and W5 list players, judged by the recorded outcome
├── credential_judges.rs           W11 and W12's judges: staged, revoked, promoted credentials and sessions
├── credential_waves.rs            W11 host agent and W12 game runtime credential rotations
├── deployment_waves.rs            W6 same-terrain, W7 cross-terrain and W8 return deployments
├── identity_waves.rs              W9 identity link, W10 kick and the kick naming an ended session
├── lost_acknowledgement_waves.rs  W13 withheld claim answer and W14 withheld result answer
├── mod.rs                         the fleet servers the waves address and the steps in the order they run
├── process_waves.rs               W1 stop, W2 start and W3 restart, with the unit and session effects
├── shared_probes.rs               the wave step, its request, a command's outcome and the unit's process
├── single_server_probes.rs        W9–W14's step and effect builders, journal, single-start and relay probes
└── wave_table.rs                  the W1–W8 table and the W9–W14 step table the plan and the action list share
```

## How it works

`wave_table.rs` holds each W1–W8 wave's number, step id, Server Control action, awaited effect
and deadline, and each W9–W14 step's wave, server, actor, action, awaited effect and deadline
(from the request row, or from the step's start for a step without one); the step's `AWAIT`
instruction and the numbered action list both come from it. The W1–W8 modules turn their rows
into one `chrome_action` step per wave with a request predicate (the first matching row since the
step began, 5 s of clock allowance included) and per-server effects whose ids are
`server<N>_<effect>`. The W9–W14 modules build `chrome_action` steps and `host_action` steps (the
credential stage, the promotion and restart, the relay's arming), whose command runs as the step
starts.

`shared_probes.rs` holds what several waves use: the command effect (pending while `queued`,
`claimed` or `executing`; contradicted by `failed`, `expired`, `cancelled` or `indeterminate`;
its time is the row's `finished_at`), the unit probe, and the new-process probe, which measures
`<step>.server<N>.pid` for the waves after it. The deployment waves measure the deployment's
scenario and confirming generation, so the config effect and the unchanged-process effect judge
only after the row that grounds them. `single_server_probes.rs` reads a unit journal from the
request row's time, holds a single unit start only once 60 s have passed after the first start
(a second one contradicts at once), and reads the relay's status for the withheld answer of the
command the request measured.

## Boundaries

- Depends on: `tools/xtask/src/commands/staging/fleet_procedure/fleet_reads.rs`,
  `fleet_cases.rs` and `judge_mapping.rs`; the step vocabulary in
  `tools/xtask/src/commands/staging/procedure_runner/step.rs`.
- Used by: `tools/xtask/src/commands/staging/fleet_procedure/mod.rs` (the plan) and
  `operator_lists.rs` (the action list).
- Rules: every probe only reads, and only a `host_action` step changes the host; a judge is a
  pure function of the observed text and the step context; a server's effect decides only that
  server's case, except W8–W14's, which decide the fleet-wide cases.
