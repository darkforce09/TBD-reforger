# Domain Features Hub (`features/`)

This directory contains the self-contained functional domains of the mission creator.

---

## Standard Feature Directory Layout
Each feature is structured as a self-contained module:
```text
features/<feature_name>/
├── README.md                      <-- Purpose, invariants, and component catalog
├── components/                    <-- Leptos UI views (buttons, dock panels, dialog tabs)
├── logic/                         <-- Algorithms, calculations, formatting helpers
└── state/                         <-- CRDT document transaction helpers
```

## Catalog of Features
- **`mission/`**: Title, save state, version history, export triggers, settings modal.
- **`assisting_tools/`**: Tactical tools (Ruler, LOS, Travel Calculator, Mortar).
- **`orbat/`**: Order of Battle, squads, slotting, outliner tree.
- **`arsenal/`**: Loadout editor, gear rules, 3D paper doll.
- **`placement/`**: Asset palette, cursor ghost preview, compositions.
- **`manipulation/`**: Selection marquee, transform gizmos, alignment chords.
- **`environment/`**: Time scrubber, weather timeline, sun angle.
- **`zones/`**: Play areas, trigger zones, polygon boundaries.
- **`tasks/`**: Mission objectives, task cards, win conditions.
- **`radio/`**: Frequency allocations, radio nets, channel plans.
- **`vehicles/`**: Vehicle roster, cargo inventory, crew slots.
- **`audio/`**: 3D sound emitters, ambient triggers.
- **`validation/`**: Mission compiler findings, rule verification.
