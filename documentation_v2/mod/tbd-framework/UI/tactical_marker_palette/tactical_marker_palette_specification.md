# TBD Reforger — Tactical Marker Palette UI Functional Specification

**System Domain:** In-Game 2D Tactical Map Overlay, Leadership Command & Control (C2), Operational Drawing & Symbology  
**Framework Alignment:** Reforger Enfusion Mod Framework (`apps/mod/tbd-framework`), Map Marker Infrastructure (`TBD_MarkerClient.c`, `TBD_MarkerService.c`), and Reforger Map Subsystem (`SCR_MapEntity`)  
**Target Roles:** Platoon Commanders, Squad Leaders, Fireteam Leaders, Vehicle Commanders, Referees  

---

## 1. Executive Summary & Purpose

The **Tactical Marker Palette** is an interactive 2D map toolbar overlay in the TBD Reforger mod that empowers leadership roles to draw, place, edit, and clear tactical operational markers directly onto the in-game topographical map.

In organized milsim, verbal radio channels become congested during contact. The Tactical Marker Palette provides an immediate visual C2 layer allowing leaders to:
- Establish phase lines, forward lines of troops (FLOT), and boundaries.
- Designate axes of advance, bounding vectors, and withdrawal routes.
- Plot NATO APP-6/MIL-STD-2525 operational symbols for spotted enemy and planned friendly positions.
- Isolate tactical clutter using strictly partitioned **communication channels** (Side, Command, Squad, Vehicle, Admin).

---

## 2. Structural Architecture & Layout Anatomy

The palette operates as a lightweight, semi-translucent floating toolbar docked along the margin of the 2D Tactical Map.

```
┌────────────────────────────────────────────────────────────────────────┐
│ [≡] TACTICAL MARKERS                    [CHANNEL: SIDE [S] ▼] [ X ]    │ <- Header & Active Channel
├────────────────────────────────────────────────────────────────────────┤
│ CHANNELS:                                                              │
│  [ [S] Side ]  [ [C] Command ]  [ [G] Squad ]  [ [V] Vehicle ] [ [A] ] │ <- Channel Tabs
├────────────────────────────────────────────────────────────────────────┤
│ MARKER TOOLS:                                                          │
│  ┌────────┬────────┬────────┬────────┬────────┬────────┐               │
│  │  DOT   │ ARROW  │  LINE  │  NATO  │  TEXT  │ ERASE  │               │
│  │  [ • ] │  [ ➔ ] │  [ ─ ] │ [ ⛝ ]  │ [ Abc ]│  [ ⌫ ] │               │
│  └────────┴────────┴────────┴────────┴────────┴────────┘               │
├────────────────────────────────────────────────────────────────────────┤
│ SUB-PALETTE: NATO SYMBOLOGY / SHAPES                                   │
│  Hostile:   [ INF ] [ ARM ] [ MECH ] [ REC ] [ AT ] [ AA ]             │
│  Friendly:  [ INF ] [ ARM ] [ MECH ] [ REC ] [ MOR] [ HQ ]             │
│  Points:    [ OBJ ] [ POI ] [ OP  ] [ RP  ] [ MED] [ AMB ]            │
├────────────────────────────────────────────────────────────────────────┤
│ LINE STYLE & WIDTH:                                                    │
│  [ ── Solid ]  [ - - Dashed ]  [ ••• Dotted ]  │ Width: [ 2px ] [ 4px ]│
├────────────────────────────────────────────────────────────────────────┤
│ COLOR PALETTE:                                                         │
│  [■ Blue]  [■ Red]  [■ Green]  [■ Yellow]  [■ Orange]  [■ White/Black] │
├────────────────────────────────────────────────────────────────────────┤
│ QUICK ACTIONS & CLEAR:                                                 │
│  [ ↶ Undo (Ctrl+Z) ]   [ Clear My Markers ]   [ Clear Channel ]        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Functional Components

### 3.1 Channel Scoping (Audience Isolation)
Markers are scoped by channel so only relevant friendly personnel receive them:
- **`[S]` Side Channel:** Visible to all friendly units in the faction (major objectives, safe areas).
- **`[C]` Command Channel:** Visible to Platoon HQ and Squad Leaders only (high-level plans, keeps riflemen maps clean).
- **`[G]` Squad / Group Channel:** Scoped to the author's immediate squad (waypoints, bounding vectors).
- **`[V]` Vehicle Channel:** Scoped to occupants of the author's vehicle.
- **`[A]` Admin / Global Channel:** Visible to all sides and referees (event staging notices).

### 3.2 Marker Tools
1. **Point / Dot (`[ • ]`):** Single-click placement for contact spotting, snipers, and quick references.
2. **Tactical Arrows (`[ ➔ ]`):** Click-and-drag directional vector for movement axes and flanking routes.
3. **Lines / Boundaries (`[ ─ ]`):** Multi-point polyline tool for phase lines (`PL`), FLOT, and boundaries.
4. **NATO Symbology (`[ ⛝ ]`):** Standard military glyphs (Infantry, Armor, Mechanized, Recon, Mortar, HQ).
5. **Text Labels (`[ Abc ]`):** Text captions and annotations with drop shadows for legibility.
6. **Eraser & Clear Tools:** Single-marker click eraser (`⌫`), `Undo` action (`Ctrl+Z`), and `Clear Channel` (leader-gated).

### 3.3 Color Swatches
- **Blue:** Friendly positions, planned friendly routes.
- **Red:** Confirmed/suspected enemy units, hostile minefields.
- **Green:** Allied/independent forces, cleared areas.
- **Yellow:** Unconfirmed contacts, hazard warnings.
- **Orange / White / Black:** Key objectives, phase lines, coordination boundaries.

---

## 4. Network & Authority Protocol

1. **Server Validation:** Marker placement RPCs are validated on the server (`TBD_MarkerService.c`). Unslotted players or riflemen cannot spam side/command channels.
2. **Zero Enemy Leaks:** The server replicates markers exclusively to players subscribed to that channel. Packets for friendly markers are never sent to enemy clients.
3. **Audit Log Trail:** All marker creations, edits, and deletions are recorded with timestamp and callsign in the Briefing Markers Log.
