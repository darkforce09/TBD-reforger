# Military Symbology & Tactical Graphics Standard

Authoritative specification for military symbology standards (NATO MIL-STD-2525 / APP-6) and in-game tactical marker systems.

---

## 1. Standard Affiliation Colorways

| Affiliation | Standard Color | Hex Code | Canvas Usage |
|---|---|---|---|
| **BLUFOR (Friendly)** | Blue | `#3b82f6` | Friendly units, squad positions, friendly rally points |
| **OPFOR (Hostile)** | Red | `#ef4444` | Confirmed enemy positions, hostile vehicles, kill zones |
| **Independent (Allied/Neutral)** | Green | `#22c55e` | Neutral assets, allied militias, civilian structures |
| **Unknown / Hazard** | Yellow | `#facc15` | Suspected enemy contacts, minefields, UXO hazards |
| **Tactical Coordination** | Orange / White | `#f97316` / `#ffffff` | Phase lines, boundaries, coordination points |

---

## 2. NATO MIL-STD-2525 Unit Icons

The map engine renders vector and MSDF icon frames conforming to standard tactical echelon and role modifiers:
- **INF** (Infantry): Saltire diagonal cross within rectangle frame.
- **ARM** (Armor / MBT): Oval track within rectangle frame.
- **MECH** (Mechanized Infantry): Combined track and saltire cross.
- **REC** (Reconnaissance): Single diagonal stripe bottom-left to top-right.
- **AT** (Anti-Tank): Inverted 'V' chevron.
- **AA** (Anti-Air): Dome arch at bottom of frame.
- **MOR** (Mortar / Artillery): Solid dot within frame.
- **HQ** (Command Element): Vertical flag pole extending downward from bottom-left corner.

---

## 3. In-Game Tactical Marker Subsystem (`TBD_MarkerService.c`)

The in-game 2D map toolbar provides players and commanders with tactical drawing primitives.

### Channel Scoping & Network Isolation
Markers are strictly isolated by communications network channel:
- `[S] Side`: Visible to all players in the faction.
- `[C] Command`: Visible only to Platoon Commander and Squad Leaders.
- `[G] Squad`: Visible only to members of the author's squad.
- `[V] Vehicle`: Visible only to occupants of the current vehicle.
- `[A] Admin`: Visible globally to match referees.

> [!SECURITY]
> Server-side validation filters marker broadcast RPCs by faction ID. Hostile clients never receive friendly marker network packets.

### Tool Palette:
1. `Point / Dot [ • ]`: Spot markers, rally points, casualty locations.
2. `Tactical Vector Arrow [ ➔ ]`: Squad movement orders, axes of advance.
3. `Polyline / Boundary [ ─ ]`: Forward Line of Own Troops (FLOT), phase lines.
4. `NATO Symbol [ ⛝ ]`: Standard unit frame placement with echelon modifiers.
5. `Text Label [ Abc ]`: Callout annotations, target designations, MGRS notes.
6. `Eraser`: Deletes owned markers (or any marker if referee/squad leader).
