# Gamemode and objectives architecture

**Status:** Proposed design  
**Scope:** `tbd-framework/Scripts/Game/Gamemode/` subsystem  
**Context:** TBD Reforger platform monorepo  

Detailed architectural specification for restructuring `Gamemode/` into an intuitive, composable
layout that cleanly separates generic engine plumbing from pluggable objective types.

---

## 1. Architectural Philosophy: Composable vs. Monolithic

### 1.1. Monolithic Framework Pattern (Anti-Pattern)
In legacy Reforger frameworks (such as CRF), game modes are structured as monolithic source files.
For example, `Rush/CRF_Rush_Game.c` spans over 3,500 lines, tightly interleaving safe-start logic,
countdown timers, zone trigger registration, score tracking, HUD updates, victory evaluation, and
hardcoded weapon loadouts into a single class.

This monolithic approach has severe flaws:
- Violates the Single Responsibility Principle and the monorepo's 500-line file limit.
- Impossible to mix and match mechanics across different scenario kinds.
- Fragile regression testing: a change to capture zone logic risks breaking unrelated match timers.

### 1.2. TBD Composable Objective Model
TBD Reforger uses a composable, data-driven mission model driven by the web CAD Mission Creator
(`apps/website/frontend/src/v2/apps/editor/`). Scenarios do not run a single hardcoded "mode"; instead,
a scenario composes multiple discrete objective instances into a unified match flow:
- Example: 1 Capture Zone + 2 Demolition Caches + 1 Holdout Phase.
- Objectives evaluate independently or in dependency chains (e.g. Cache 2 unlocks only after Zone 1
  is captured).
- Match progression and win conditions dynamically resolve based on active objective states.

---

## 2. Restructured `Gamemode/` Topology

```text
Scripts/Game/Gamemode/
├── README.md                       # Gamemode subsystem overview & lifecycle contracts
│
├── Orchestrator/                   # Whole-match flow, state transitions & heartbeat
│   ├── README.md
│   ├── TBD_FrameworkManager.c      # Match coordinator & subsystem lifecycle driver
│   ├── TBD_FrameworkRollCall.c     # Client synchronization & slotting roll call
│   ├── Flow/                       # JIP policies, mission flow reports
│   │   ├── README.md
│   │   ├── TBD_EJipPolicy.c
│   │   ├── TBD_JipPolicy.c
│   │   ├── TBD_MissionFlow.c
│   │   └── TBD_MissionFlowReport.c
│   ├── Heartbeat/                  # 1000ms server tick & cyclic health evaluation
│   │   ├── README.md
│   │   └── TBD_RuntimeHeartbeat.c
│   └── Stage/                      # Stage environments, elimination, round clocks
│       ├── README.md
│       ├── TBD_EndBanner.c
│       ├── TBD_FactionElimination.c
│       ├── TBD_LoadingGate.c
│       ├── TBD_RoundClock.c
│       └── TBD_StageEnvironment.c
│
├── Stages/                         # Match progression phases
│   ├── README.md
│   ├── TBD_EGameStage.c            # Stage enum (PRE_GAME, SAFESTART, LIVE, POST_GAME)
│   ├── Safestart/                  # Safe-start countdown, weapon locks & boundaries
│   │   ├── README.md
│   │   ├── TBD_SafestartManager.c
│   │   ├── TBD_SafestartProtection.c
│   │   └── TBD_SafestartWatchdog.c
│   └── WinConditions/              # Victory / defeat criteria & evaluation rules
│       ├── README.md
│       ├── TBD_WinConditionEvaluator.c
│       ├── TBD_WinConditionModes.c
│       └── TBD_WinConditionsStruct.c
│
└── Objectives/                     # Composable objective subsystem
    ├── README.md                   # Objective subsystem architecture & authoring guide
    │
    ├── Engine/                     # Generic engine plumbing & lifecycle
    │   ├── README.md               # Objective engine contracts & update loop
    │   ├── TBD_ObjectiveRegistry.c     # Active objective index & ID lookup
    │   ├── TBD_ObjectiveRuleResolver.c # Dependency resolution & rule evaluation
    │   ├── TBD_ObjectiveProgression.c  # Progress percentage calculation
    │   ├── TBD_ObjectiveHudPublisher.c # Client replication of objective HUD states
    │   ├── TBD_ObjectivesComponent.c   # GameMode entity component hook
    │   ├── TBD_TaskStateMachine.c      # State machine driver for objective tasks
    │   ├── TBD_Task.c                  # Core task model
    │   ├── TBD_TaskSchedule.c          # Task scheduling & triggers
    │   ├── TBD_TaskStruct.c            # Task wire structures
    │   ├── TBD_ObjectiveEndConditions.c# Victory triggers bound to objectives
    │   ├── TBD_ObjectiveEntityReader.c # World entity query bridge
    │   ├── TBD_ObjectiveRulesReader.c  # Scenario JSON rule deserializer
    │   ├── TBD_ObjectiveTypedBinder.c  # Factory instantiating typed objective handlers
    │   └── Model/                  # Shared objective enums & data structures
    │       ├── README.md
    │       ├── TBD_EObjectiveKind.c
    │       ├── TBD_EObjectiveOnEmpty.c
    │       ├── TBD_EObjectiveRole.c
    │       ├── TBD_Objective.c
    │       └── TBD_ObjectiveText.c
    │
    └── Types/                      # Pluggable objective types (modular & discoverable)
        ├── README.md               # Guide to implementing new objective types
        ├── Capture/                # Zone & flag capture logic
        │   ├── README.md
        │   └── TBD_ObjectiveCapture.c
        ├── Destroy/                # Cache & physical target demolition
        │   ├── README.md
        │   ├── TBD_ObjectiveDestroy.c
        │   └── TBD_ObjectiveDestroyTargets.c
        ├── HoldUntil/              # Defense & wave holdout timer
        │   ├── README.md
        │   └── TBD_ObjectiveHoldUntil.c
        └── HVT/                    # High Value Target VIP extraction & defense
            ├── README.md
            └── TBD_ObjectiveHVT.c
```

---

## 3. Subsystem Breakdown: `Engine/` vs. `Types/`

### 3.1. `Objectives/Engine/` (The Generic Subsystem)
The `Engine/` folder houses the foundational machinery that manages objective lifecycles regardless
of the specific objective type:
- **`TBD_ObjectiveRegistry`**: Manages the collection of all instantiated objectives for a scenario,
  providing lookup by integer or string ID.
- **`TBD_ObjectiveTypedBinder`**: The factory that inspects the mission specification and instantiates
  the appropriate typed handler from `Types/`.
- **`TBD_ObjectiveRuleResolver`**: Evaluates complex dependency logic (e.g. "Objective B becomes active
  when Objective A is COMPLETED").
- **`TBD_ObjectiveProgression`**: Aggregates overall mission completion metrics for scoreboard and
  telemetry reporting.
- **`TBD_ObjectiveHudPublisher`**: Encapsulates RPC broadcasting so that connected players see live HUD
  markers and task state notifications.
- **`Model/`**: Shared data definitions (`TBD_EObjectiveKind`, `TBD_Objective` base class) inherited
  by all objective implementations.

### 3.2. `Objectives/Types/` (Pluggable Objective Handlers)
Each specific objective kind is encapsulated in its own discoverable directory under `Types/`:
- **`Capture/`**: Area capture points, radius triggers, contesting factions, and capture progress
  interpolation (`TBD_ObjectiveCapture.c`).
- **`Destroy/`**: Physical demolition targets (weapon caches, radar towers, radio relays) tracking
  damage and destruction events (`TBD_ObjectiveDestroy.c`, `TBD_ObjectiveDestroyTargets.c`).
- **`HoldUntil/`**: Timed holdout or defensive wave objectives where a defending faction must retain
  control until a countdown expires (`TBD_ObjectiveHoldUntil.c`).
- **`HVT/`**: High-Value Target extraction or assassination mechanics, tracking VIP character health,
  hostage states, and extraction zones (`TBD_ObjectiveHVT.c`).

---

## 4. Guide: Implementing a New Objective Type

The modular architecture makes adding new objective types straightforward and safe:
1. **Create Directory**: Add `Objectives/Types/<KindName>/` with a dedicated `README.md`.
2. **Subclass Base**: Create `TBD_Objective<KindName>.c` extending `TBD_Objective`.
3. **Implement Virtual Methods**:
   - `OnActivated()`: Initialize zone triggers or entity listeners.
   - `OnTick(float deltaTime)`: Periodic state evaluation.
   - `OnCompleted()`: Trigger resolution rewards and notify engine.
4. **Register in Binder**: Register the new enum value in `TBD_EObjectiveKind` and bind the factory
   instantiation in `TBD_ObjectiveTypedBinder.c`.
5. **Verify File Constraints**: Ensure the new implementation stays strictly under 500 lines.

---

## 5. Related Documentation

- [Master overview](01_master_overview.md) — mod reorganization synthesis.
- [Framework scripts flattening](05_framework_scripts_flattening.md) — `Scripts/Game/` domain organization.
- [Tooling and CI impact](07_tooling_and_ci_impact.md) — compilation and world boot gates.
- [Migration roadmap](08_migration_roadmap.md) — step-by-step rollout plan.
