# T-181 — TBD Framework: the Arma-3-parity event mod

**Hub doc.** Open this to know what the program is and what to run next.
Registry: [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) · North star:
[`TBD_MOD_DESIGN.md`](TBD_MOD_DESIGN.md) (T-181.4) · Capability verdicts:
[`capability_verdicts.tsv`](capability_verdicts.tsv)

## What this program is

**TBD is rebuilding the Arma 3 mission workflow inside Arma Reforger.**

| Arma 3 | TBD Reforger |
|---|---|
| Eden editor | the website Mission Creator (`apps/website/frontend`) |
| `.pbo` mission file | **compiled mission JSON** (`GET /api/v1/missions/:id/compiled`) |
| Engine-native lobby, briefing, slotting, respawn, spectator | **Reforger ships none of it — the mod must build all of it** |

That last row is the whole reason this is a program and not a feature. **CRF** (Arma Public
License, 266 `.c` / 71,606 LOC, gitignored at `apps/mod/crf_framework`) is the proof it can be
done and is the reference oracle — **indexed, never vendored**.

**Locked decisions:** events are **ONE LIFE** (death is terminal; an admin can respawn a glitch
death). Lobby UI follows **macOS methodology** — frictionless, direct-manipulation — rendered in
the website's **Aegis** tokens (`apps/website/frontend/style/aegis.css`). Full AAR/statistics is
**DEFERRED by operator word**.

## Run this

```bash
make mod-compile             # Enfusion compile gate — ~1.3s, native, no Workbench
make mod-compile-selftest    # prove the gate still catches a broken .c
make verify-capability       # fails if any CRF capability has no TBD verdict
make enf-index               # rebuild the CRF symbol index

cargo run -q -p tbd-tools --bin enf -- lookup UpdateSlotPlayerID
cargo run -q -p tbd-tools --bin enf -- dirs --depth 5 --min 60
```

**Everything runs on the host.** Agent shells run inside a `debian:12` container with no C
toolchain, so builds and game binaries route through `scripts/lib/hostrun.sh`
(`distrobox-host-exec`). An in-container `cargo build` fails with `linker cc not found`, and a
host-built binary fails in-container with `GLIBC_2.39 not found`. **Neither means the repo is
broken** — a session once "fixed" that misdiagnosis and destroyed 2.6 GB of build artifacts.

## The two compile lanes

| Lane | Runs on | Serial? | Covers |
|---|---|---|---|
| **Fast** | native `ArmaReforgerServer` | **No — parallel** | script compile, script runtime, headless MP, JIP, determinism |
| **Slow** | Workbench (Proton, :5775) | Yes, one instance | world/prefab authoring, resource DB, PIE visual checks |

Default to the fast lane. Measured: compile **780 ms**, whole gate **1.3 s** — versus a
90–120 s Workbench restart that also needed a human to click "Open".

## Stage 1 — FACTORY (shipped)

| Slice | What | Gate |
|---|---|---|
| T-181.0 | `scripts/lib/hostrun.sh` host-aware execution | `ticket check` green from the container |
| T-181.1 | `scripts/mod/compile.sh` + `make mod-compile` | 0 clean / 1 + `file:line` broken |
| T-181.2 | `enf` bin + CRF index (`.ai/artifacts/enf-index/crf_*.tsv`) | real symbol resolves, invented one does not |
| T-181.2.1 | capability matrix + UNTRIAGED gate | 57 capabilities, zero untriaged |

| T-181.3 / .3.2 | vanilla carve + by-name pak extract | 5 prove-out classes resolve |
| T-181.3.1 | official BI Script API mirror (7,990 classes) | signatures for everything |
| T-181.3.3 | **full vanilla source WITH BODIES** via AR Explorer | `SCR_BaseGameMode.c` resolves |
| T-181.4 | `TBD_MOD_DESIGN.md` + `@idx` citation gate + CRF-leak gate | hallucinated symbol exits 1 |

**The vanilla oracle has four lanes** — see [`vanilla_carve_coverage.md`](vanilla_carve_coverage.md).
Lane 4 (AR Explorer Doxygen source, same game version, method bodies included) supersedes the
others; the pak-compression codec was never cracked and no longer needs to be.

## Stage 2 — the mod spine

**Shipped:** gamemode state machine (mostly pre-existing) · one-life death model · admin respawn ·
JSON win conditions · slotting authority (claim/release/roster + one-life integrity).

**In flight (wave 1, parallel agents):** UI framework · JSON loadouts · mission validation.

**Remaining:** lobby/slot picker UI → briefing screen → spectator → JIP/reconnect → E2E gate.

`T-181.5` (Workbench `-gproj`) is **deprioritised** — the native compile lane removed most of the
reason to open Workbench. `T-181.6` (auto-runner) is superseded in practice by the command-center +
slice-agent model below.

Stages are **dependency-gated, not calendar-gated**: a stage ends when its gate goes green, and
the next starts immediately. `./scripts/ticket next` is authoritative.

## Execution model — command center + slice agents

This chat is the **command center**. It owns the registry, integration, and sequencing. Each slice is
delegated to a **full-capability Opus subagent** with a self-contained prompt, so the main context
stays clear and each slice gets the whole context window to reason in.

**Why this is safe now and was not before the factory.** A slice agent can *prove* its work instead
of claiming it:

| Gate | What it proves |
|---|---|
| `bash scripts/mod/compile.sh` | the Enfusion actually compiles (~1.3 s, no Workbench) |
| `enf lookup <Symbol>` | the API being called genuinely exists |
| `make verify-no-crf-leak` | no Arma-Public-License code leaked into prod |
| `make verify-capability` | no capability silently forgotten |
| `make verify-oracle` | no invented `file:line` in docs |

**Rules for slice agents** (put these in every prompt):
1. **Host-aware.** Prefix builds/game binaries with `distrobox-host-exec`. Explain WHY, or the agent
   will "fix" a working toolchain — that already cost 2.6 GB once.
2. **Never self-ship.** Agents implement + compile-verify + report. The command center owns
   `registry.json` and all status transitions.
3. **Batch by file-disjointness, not by count.** The parallelism limit is file collisions, not
   Workbench — the compile gate is parallel-safe (unique temp profile per run, process-group kill
   scoped to that run). `TBD_SpawnManager.c` and `TBD_FrameworkManager.c` are the contended files;
   never give two concurrent agents write access to the same one. Tell each agent which files other
   agents hold, and to REPORT needed hooks rather than edit across the line.
4. **Point at docs, don't inline them.** `TBD_MOD_DESIGN.md` + this hub + `capability_verdicts.tsv`
   carry the binding context.
5. **Demand honesty about verification depth** — compile-verified is not runtime-verified. Anything
   visual needs the operator's eyes; no tool here returns a framebuffer.

## How we work here

- **CRF-first.** Query the index before designing. Don't reinvent the wheel; don't ship its look.
- **Prove Enfusion from source.** Index row, carved vanilla, or `api_search` — never memory.
  An agent summarising `CRF_SlottingManager.c` invented four APIs (`RequestSlotChange`,
  `ReleaseSlot`, `GetInstance`, a wrong base class) and missed `RplSave`/`RplLoad`. That is why
  symbols are extracted mechanically and prose cites index rows.
- **Make it work → right → fast.** Working is not half-assed.
- **No silent deferrals.** The only deferral list is §Deferrals in `TBD_MOD_DESIGN.md`, and it
  carries the operator's own words.

## Absorbed work

**T-068.13** (production LOBBY / slot picker) is **superseded by T-181.9.1**. Scope moved, not
dropped: the picker ships on the new gamemode state machine. `ClaimSlot` in `TBD_SpawnManager`
is already the backend it binds to; `m_bAutoDeploy` turns off the PIE auto-wave when it lands.

## Landmines (measured — do not relearn)

- `[RplProp(onRplName:)]` fires automatically **only on the proxy**; authority must invoke its
  own handler (CRF says so at `CRF_Gamemode.c:8`).
- The join hook is `OnPlayerAuditSuccess(int)`, **not** `OnPlayerConnected`
  (`CRF_Gamemode.c:411`). Disconnect takes 3 args (`:465`).
- `SetInitialMainEntity` possesses a body but is **not a spawn** — the client hangs on the
  loading screen. Use the vanilla POSSESS request.
- The dedicated server **exits 0 even when compilation fails**. Read the logs, never `$?`.
- `int x = ;` compiles clean — Enfusion is lenient. Undefined symbols are what actually error.
- Enforce `set`/`array.Remove` is **by index**.
- `wb_reload` never recompiles; Workbench must restart per compile (the fast lane avoids this).
