# Enforce Script Architecture Hub (`Scripts/Game/TBD`)

This directory contains the runtime codebase for the TBD Framework in Arma Reforger.

---

## The 6-Domain Architecture

```text
Scripts/Game/TBD/
├── Core/                      <-- Base zero-dependency utilities, logging, registry alias resolution
├── API/                       <-- REST communication with website-api & identity link
│
├── Gamemode/                  <-- MATCH RULES, STAGES, & ORCHESTRATION
│   ├── Orchestrator/          <-- Round clock, stage state machine, match conductor (TBD_FrameworkManager.c)
│   ├── Stages/                <-- Safestart, stage progression, win condition evaluator
│   └── Objectives/            <-- Dynamic task state machines, objective registry & rules
│
├── Systems/                   <-- IN-WORLD SIMULATION, DATA INGESTION, & TOOLS
│   ├── Mission/               <-- JSON schema data contracts, loaders, validators, weather & scatter
│   ├── Spawning/              <-- Spawner logic, respawn system, spawn handler
│   ├── Loadouts/              <-- Gear & inventory equipping
│   ├── Audio/                 <-- 3D audio source entities & music cues
│   ├── Zones/                 <-- Play areas, geometry, triggers, and volumes
│   ├── Markers/               <-- 2D map marker client, controller, icons
│   ├── Radio/                 <-- Frequency plan, tuner, VOIP bridge, controller
│   └── AI/                    <-- Waypoints & group state runtime
│
├── Session/                   <-- PLAYER & REFEREE INTERACTION SYSTEMS
│   ├── Lobby/                 <-- ORBAT slotting, PreSlotCamera, Controller, UI Screen
│   ├── MissionSelector/       <-- Scenario browser, MissionBrowser RPCs, ScenarioRouter
│   ├── Briefing/              <-- Tactical briefing Service, Client, Controller, UI Screen
│   ├── Spectator/             <-- Camera, Host targets, Controller, Forensics UI Screen
│   ├── Admin/                 <-- Mission Control Service, Audit, Commands, UI Screen
│   └── PostGame/              <-- AAR & End screen debrief UI Screens
│
└── UI/                        <-- SHARED UI COMPONENT LIBRARY ONLY
    ├── Core/                  <-- Framework base (MenuBase, MenuStack, ShellScreen, UITheme)
    ├── Common/                <-- Shared row/chip handlers
    ├── Hud/                   <-- Generic persistent HUDs (ObjectiveHud, TaskHud)
    └── Mock/                  <-- Shared mock data for UI prototyping
```

---

## Core Principles & Design Patterns

### 1. The Data / Service / Controller / Client Pattern
Each feature in `Session/` and `Systems/` follows a decoupled architecture:
- **`TBD_XData.c`:** Plain model classes & structs (compiled on both client and server).
- **`TBD_XService.c`:** Server-only business logic; builds payloads from mission documents.
- **`TBD_XController.c`:** `modded class SCR_PlayerController` RPC pairs between client and server.
- **`TBD_XClient.c`:** Client-only cache + `ScriptInvoker` signals that UI screens bind to.
- **`TBD_XComponent.c`:** Lifecycle host extending `SCR_BaseGameModeComponent`.

### 2. Gamemode (Rules & Orchestration) vs. Systems (Tools & Simulation)
- **`Gamemode/`** acts as the match referee and conductor. It coordinates round stages, runs the countdown clock, evaluates dynamic mission objectives (capture, hold, destroy), and decides victory conditions.
- **`Systems/`** contains background simulation mechanics, data ingestion, and tools that run continuously in the 3D world (spawning bodies, dressing inventories, deserializing missions, spatial trigger evaluations, radio channels, AI waypoints). Systems do not decide game outcomes; they provide tools that Gamemode and Session query.

### 3. Session vs. Systems
- **`Session/`** contains human-facing operational systems that govern the player's event journey (selecting a mission, slotting in the ORBAT, viewing pre-slot cameras, reading briefings, refereeing, spectating upon death, reviewing the AAR).

### 4. UI Component Library Separation
`UI/` owns only the shared visual building blocks (`UIButton`, `TBD_ListBox`, `MenuBase`). Screen controllers live directly alongside their domain logic inside `Session/<Feature>/UI/` and consume components from `UI/`.
