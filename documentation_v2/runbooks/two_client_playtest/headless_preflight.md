**Status:** live

# Rehearse the playtest mission headless

Boots the dedicated server with no clients on a loadout-carrying
[mission](/documentation_v2/glossary.md#mission) — first a golden, then the exact bytes of the
mission you authored — and reads whether the loadout pass lets the round open. It catches the
failure most likely to end the session, a mission whose bodies nobody can play, in about ten
minutes and without a second player. Run it alone after
[preparing the stack](/documentation_v2/runbooks/two_client_playtest/stack_and_mission.md).

## Prerequisites

- The prerequisites of [Stack and mission](/documentation_v2/runbooks/two_client_playtest/stack_and_mission.md).
- For step 3, the API running and the authored mission's id as `MID`.

## How to judge a boot

Judge each boot on its log lines, never on `WORLD BOOT: PASS` or `FAIL`. The verdict's warning
ratchet compares the count in `[TBD][Validate] mission result=PASS errors=0 warnings=N` with the
mission's budget in `.world-boot-warning-baseline` at the repository root. The budgets of both
goldens are 0, and the validator's unconsumed-key checks warn on both: `environment` and
`orbat.roles.radio` on `slot-loadout-coverage`, plus `layers` on `bridgehead-at-levie`. So expect

```text
  FAIL  validator warnings rose: <N> > baseline 0 for <missionId>
WORLD BOOT: FAIL
```

with `<missionId>` `msn_5c1de7` or `msn_8f3a2c` (`compiled` on the lane of step 3, budget 1).
That `FAIL` is not about the session when all three hold:

1. the failing line says `validator warnings rose:`; any other `FAIL` line is real;
2. the log's `[TBD][Validate] mission result=` line reads `result=PASS errors=0`;
3. every `[TBD][Validate] WARNING` line names one of the unconsumed-key subjects
   (`TBD_MissionValidator.CheckUnconsumedKeys`):

   | Subject | What the mod does not apply |
   |---|---|
   | `environment` | weather and time of day |
   | `settings` | a `respawn` other than `none`, or `nightVision: true` |
   | `layers` | decoration layer aliases |
   | `factions.tickets` | a positive respawn pool |
   | `orbat.roles.radio` | role-scoped net assignment (nets are delivered per faction) |

   A warning naming anything else is worth stopping for.

The pass signal is the pair `[TBD][Slots] loadout settle complete` and `[TBD] Stage → LOBBY`.
Match the prefix of the settle line; its counts vary with the mission.

## Steps

1. Boot the coverage golden and keep its log. The loadout pass needs about 3 to 4 seconds after
   the mission loads, and the harness stops the server `TBD_WORLDBOOT_SETTLE` seconds (default 4)
   after the `mission result=` line, so raise it or the log ends mid-pass and looks clean.

   ```bash
   TBD_WORLDBOOT_SETTLE=25 cargo xtask mod world-boot --mission=slot-loadout-coverage --keep-logs
   ```

   Expected: `run dir kept: <run dir>`, then the verdict described above.

2. List the loadout verdict and the stages from the kept log.

   ```bash
   grep -E 'loadout settle|loadout delivery REFUSED|loadout SHORTFALL|loadout pass complete|loadout DEGRADED|loadout INCOMPLETE|Stage →' <run dir>/profile/logs/logs_*/console.log
   ```

   Expected: `[TBD][Slots] loadout settle complete — <N> application(s), 0 unplayable, <M> with a shortfall — spawn open`
   (`TBD_SpawnManager.c`), then `[TBD] Stage → LOBBY`. A non-zero `<M>` is fine (see the table
   below).

3. Boot the authored mission's compiled bytes, fetched from the running API: the same document the
   game server will load, parsed by the real Enfusion parser.

   ```bash
   TBD_WORLDBOOT_SETTLE=25 cargo xtask mod world-boot --compiled=$MID --keep-logs
   ```

   Expected: `run dir kept: <run dir>`; step 2's grep over that run dir shows the settle line and
   `Stage → LOBBY`. Besides the ratchet, this lane asserts the seeded fixture's four weapons
   (exactly four `[TBD][Equip] slot=<0-3> weapon=… result=ok` lines), so an authored mission with
   more or fewer armed slots also prints `FAIL  four-weapon equip`; judge the boot on the log.

## Verify

The authored mission's log from step 3 carries both lines:

```bash
grep -E '\[TBD\]\[Slots\] loadout settle complete|\[TBD\] Stage → LOBBY' <run dir>/profile/logs/logs_*/console.log
```

Expected: one settle line with `0 unplayable` and one `[TBD] Stage → LOBBY`: a loadout-carrying
mission reaches the lobby, and the session can open.

## Reading the loadout lines

A TBD loadout `SCRIPT (E)` line means the round will not open; a `SCRIPT (W)` line means it will,
with someone carrying less than authored.

| Line | Verdict | Meaning |
|---|---|---|
| `[TBD][Slots] loadout settle complete — … 0 unplayable, M with a shortfall — spawn open` | go | every body is playable |
| `[TBD][Slots] loadout SHORTFALL on M of N slot(s) — the session IS open …` | go, note it | those players are dressed and armed, carrying an item elsewhere than authored, or fewer of it; the admin trail (`#tbd audit`) records `LOADOUT: …` |
| `[TBD][Loadout][Slot] slot=<id> <what> DEGRADED item=… — …` | go, note it | per item: the kit lacks the named container, so the item went into any storage that takes it |
| `[TBD][Slots] loadout delivery REFUSED at spawn boundary — N slot body(ies) are UNPLAYABLE — LOBBY/deploy will not open` | stop | an authored prefab that does not load, a storage or weapon slot the character does not have, or a garment that would not go on; the line names the slot and the item |
| `[TBD][Loadout][Slot] slot=<id> loadout INCOMPLETE …` | stop when an ERROR | the per-slot detail behind a refusal |

A refusal holds every player in LOADING, deliberately: fix the mission document, save a new
version and submit it again.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| the log ends before any `loadout settle` line and shows no error | the harness stopped the server before the pass settled | set `TBD_WORLDBOOT_SETTLE=25` |
| `FAIL  validator warnings rose:` | the warning ratchet (see "How to judge a boot") | apply the three-part test; do not widen the baseline |
| `FAIL  four-weapon equip` on `--compiled=$MID` | the assertion counts the seeded fixture's four weapons | judge on the settle and stage lines |
| `loadout delivery REFUSED at spawn boundary` | an unplayable slot body | fix the named slot and item in the Mission Creator; resubmit |
| `WORLD BOOT: ENV FAIL — …` on step 3 | the API gave no answer | start `cargo xtask mk rust-api` |

## Related

- [World boot](/tools_v2/xtask/src/commands/mod_ops/world_boot/README.md) — the harness, its
  environment variables and exit codes.
- [Playtest server](/documentation_v2/runbooks/two_client_playtest/playtest_server.md) — the next
  runbook: the joinable server.
- [Loadouts](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/README.md) — the loadout
  pass that prints these lines.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
